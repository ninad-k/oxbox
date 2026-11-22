<#
.SYNOPSIS
Samples only aggregate owner-PID TCP/UDP endpoint counts for a pinned QA binary.
.DESCRIPTION
Windows only. Does not launch/stop apps, trace DNS/packets, elevate, enable logs or
change OS settings. Start this observer before a separate controlled app launch.
Both executable and fresh evidence directory must be below this checkout's target/.
Snapshots can miss short flows/processes; UDP rows are bindings, not packets.
#>
param(
    [Parameter(Mandatory = $true)][string]$ExpectedExecutable,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [Parameter(Mandatory = $true)][ValidatePattern('^[0-9a-fA-F]{64}$')][string]$ExpectedSha256,
    [ValidateRange(30, 600)][int]$DurationSeconds = 180,
    [ValidateRange(20, 1000)][int]$IntervalMilliseconds = 50
)
$ErrorActionPreference = 'Stop'
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'This read-only endpoint observer requires Windows.' }
function Read-Sha256([string]$Path) {
    $hashStream = [System.IO.File]::OpenRead($Path)
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($algorithm.ComputeHash($hashStream))).Replace('-', '').ToLowerInvariant() }
    finally { $algorithm.Dispose(); $hashStream.Dispose() }
}
function Test-ObservationValidity(
    [bool]$CompletedNormally,
    [int]$SampleFailures,
    [int]$MarkerFailures,
    [bool]$TargetObserved,
    [bool]$HashUnchanged
) {
    return ($CompletedNormally -and $SampleFailures -eq 0 -and $MarkerFailures -eq 0 -and $TargetObserved -and $HashUnchanged)
}
function Assert-NoReparseAncestor([string]$Path) {
    $current = [System.IO.Path]::GetPathRoot($Path)
    foreach ($part in $Path.Substring($current.Length).Split([System.IO.Path]::DirectorySeparatorChar)) {
        if (-not $part) { continue }
        $current = Join-Path $current $part
        try { $attributes = [System.IO.File]::GetAttributes($current) }
        catch [System.IO.FileNotFoundException] { break }
        catch [System.IO.DirectoryNotFoundException] { break }
        if (($attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw 'Observer paths cannot traverse reparse points.'
        }
    }
}
function Read-PhaseMarker([string]$MarkerPath) {
    $stream = $null
    $reader = $null
    try {
        $sharing = [System.IO.FileShare]::ReadWrite -bor [System.IO.FileShare]::Delete
        $stream = [System.IO.FileStream]::new($MarkerPath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, $sharing)
        if ($stream.Length -gt 256) { return $null }
        $reader = [System.IO.StreamReader]::new($stream, [System.Text.Encoding]::UTF8)
        return $reader.ReadToEnd().Trim()
    } finally {
        if ($null -ne $reader) { $reader.Dispose() }
        elseif ($null -ne $stream) { $stream.Dispose() }
    }
}
$expectedPath = [System.IO.Path]::GetFullPath($ExpectedExecutable)
$workspaceRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$targetRoot = [System.IO.Path]::GetFullPath((Join-Path $workspaceRoot 'target'))
if (-not $expectedPath.StartsWith($targetRoot + [System.IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or
    [System.IO.Path]::GetFileName($expectedPath) -ine 'oxbox-native.exe') {
    throw 'The executable must be an exact oxbox-native.exe inside this checkout target directory.'
}
$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)
$allowedRoot = [System.IO.Path]::GetFullPath((Join-Path $targetRoot 'native-privacy-qa'))
if (-not $outputPath.StartsWith($allowedRoot + [System.IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'The output directory must be inside target/native-privacy-qa.'
}
Assert-NoReparseAncestor $expectedPath
Assert-NoReparseAncestor $outputPath
if (-not [System.IO.File]::Exists($expectedPath)) { throw 'The exact QA executable does not exist.' }
$executableSha256 = Read-Sha256 $expectedPath
if (-not $ExpectedSha256.Equals($executableSha256, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'The QA executable hash does not match the verified production artifact.'
}
if ([System.IO.Directory]::Exists($outputPath) -or [System.IO.File]::Exists($outputPath)) { throw 'Use a fresh output directory; existing evidence is preserved.' }
[System.IO.Directory]::CreateDirectory($outputPath) | Out-Null

# Read-only, non-elevated process and IP Helper table queries. No app launch,
# packet capture, ETW session, firewall or OS/browser configuration changes.
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Net;
using System.Runtime.InteropServices;
using System.Text;
public static class OxboxEndpointObserver {
    [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
    public struct ProcessEntry {
        public uint Size, Usage, Id;
        public UIntPtr DefaultHeap;
        public uint ModuleId, Threads, ParentId;
        public int Priority;
        public uint Flags;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst=260)] public string Name;
    }
    public class ProcessNode { public int Id, Parent; public string Name; }
    public class Endpoint { public int Pid, State; public bool Tcp, V6, Loopback, NoRemote, Private; public string Key; }
    [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr CreateToolhelp32Snapshot(uint flags, uint pid);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool Process32FirstW(IntPtr snapshot, ref ProcessEntry entry);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool Process32NextW(IntPtr snapshot, ref ProcessEntry entry);
    [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool QueryFullProcessImageNameW(IntPtr process, uint flags, StringBuilder name, ref uint size);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool GetProcessTimes(IntPtr process, out long creation, out long exit, out long kernel, out long user);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    [DllImport("iphlpapi.dll", SetLastError=true)] static extern uint GetExtendedTcpTable(IntPtr table, ref int length, bool ordered, int family, int tableClass, uint reserved);
    [DllImport("iphlpapi.dll", SetLastError=true)] static extern uint GetExtendedUdpTable(IntPtr table, ref int length, bool ordered, int family, int tableClass, uint reserved);
    public static ProcessNode[] Processes() {
        var output = new List<ProcessNode>();
        IntPtr snapshot = CreateToolhelp32Snapshot(2, 0);
        if (snapshot == new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error());
        try {
            var entry = new ProcessEntry(); entry.Size = (uint)Marshal.SizeOf(typeof(ProcessEntry));
            if (!Process32FirstW(snapshot, ref entry)) throw new Win32Exception(Marshal.GetLastWin32Error());
            do { output.Add(new ProcessNode { Id=(int)entry.Id, Parent=(int)entry.ParentId, Name=entry.Name }); }
            while (Process32NextW(snapshot, ref entry));
            int error=Marshal.GetLastWin32Error(); if (error != 18) throw new Win32Exception(error);
        } finally { CloseHandle(snapshot); }
        return output.ToArray();
    }
    public static string Path(int pid) {
        IntPtr handle=OpenProcess(0x1000, false, pid); if(handle==IntPtr.Zero)return null;
        try { var text=new StringBuilder(32768); uint length=(uint)text.Capacity; return QueryFullProcessImageNameW(handle,0,text,ref length)?text.ToString():null; }
        finally { CloseHandle(handle); }
    }
    public static long Started(int pid) {
        IntPtr handle=OpenProcess(0x1000,false,pid); if(handle==IntPtr.Zero)return 0;
        try {long created,exit,kernel,user;return GetProcessTimes(handle,out created,out exit,out kernel,out user)?created:0;}
        finally{CloseHandle(handle);}
    }
    static bool PrivateAddress(IPAddress address) {
        if(address.IsIPv4MappedToIPv6)address=address.MapToIPv4();
        byte[] bytes=address.GetAddressBytes();
        if(bytes.Length==4)return bytes[0]==10 || (bytes[0]==172 && bytes[1]>=16 && bytes[1]<=31) || (bytes[0]==192 && bytes[1]==168) || (bytes[0]==169 && bytes[1]==254);
        return address.IsIPv6LinkLocal || (bytes[0]&0xfe)==0xfc;
    }
    static Endpoint[] ReadTable(bool tcp,bool v6) {
        int size=0; int family=v6?23:2;
        uint error=tcp?GetExtendedTcpTable(IntPtr.Zero,ref size,false,family,5,0):GetExtendedUdpTable(IntPtr.Zero,ref size,false,family,1,0);
        if(error!=0 && error!=122)throw new Win32Exception((int)error);
        for(int attempt=0;attempt<4;attempt++) {
            if(size<4)size=4; if(size>32*1024*1024)throw new InvalidOperationException("Endpoint table exceeded bounded observer memory.");
            IntPtr buffer=Marshal.AllocHGlobal(size);
            try {
                error=tcp?GetExtendedTcpTable(buffer,ref size,false,family,5,0):GetExtendedUdpTable(buffer,ref size,false,family,1,0);
                if(error==122)continue; if(error!=0)throw new Win32Exception((int)error);
                int count=Marshal.ReadInt32(buffer), stride=tcp?(v6?56:24):(v6?28:12);
                if(count<0 || (long)count*stride+4>size)throw new InvalidOperationException("Endpoint table length was invalid.");
                var output=new List<Endpoint>(count);
                for(int index=0;index<count;index++) {
                    byte[] row=new byte[stride]; Marshal.Copy(IntPtr.Add(buffer,4+index*stride),row,0,stride);
                    int pidOffset=tcp?(v6?52:20):(v6?24:8);
                    var entry=new Endpoint{Pid=(int)BitConverter.ToUInt32(row,pidOffset),Tcp=tcp,V6=v6};
                    if(tcp) {
                        int addressOffset=v6?24:12, addressLength=v6?16:4;
                        byte[] remote=new byte[addressLength];Array.Copy(row,addressOffset,remote,0,addressLength);
                        var address=new IPAddress(remote);if(address.IsIPv4MappedToIPv6)address=address.MapToIPv4();
                        entry.State=(int)BitConverter.ToUInt32(row,v6?48:0);
                        entry.Loopback=IPAddress.IsLoopback(address);entry.NoRemote=entry.State==2||address.Equals(IPAddress.Any)||address.Equals(IPAddress.IPv6Any);
                        entry.Private=PrivateAddress(address);
                        // The identity is held only in memory; no addresses or ports are exported.
                        entry.Key=entry.Pid+":"+v6+":"+address+":"+BitConverter.ToUInt32(row,v6?44:16)+":"+BitConverter.ToUInt32(row,v6?20:8);
                    }
                    output.Add(entry);
                }
                return output.ToArray();
            } finally {Marshal.FreeHGlobal(buffer);}
        }
        throw new InvalidOperationException("Endpoint table changed too often to read safely.");
    }
    public static Endpoint[] Endpoints() {
        var output=new List<Endpoint>();
        output.AddRange(ReadTable(true,false));output.AddRange(ReadTable(true,true));
        output.AddRange(ReadTable(false,false));output.AddRange(ReadTable(false,true));
        return output.ToArray();
    }
}
'@

$samplesPath = Join-Path $outputPath 'samples.jsonl'
$phasePath = Join-Path $outputPath 'phase.txt'
$stopPath = Join-Path $outputPath 'stop.txt'
$writer = [System.IO.StreamWriter]::new($samplesPath, $false, [System.Text.UTF8Encoding]::new($false))
$clock = [System.Diagnostics.Stopwatch]::StartNew()
$known = [System.Collections.Generic.Dictionary[int,long]]::new()
$rootIds = [System.Collections.Generic.HashSet[int]]::new()
$names = [System.Collections.Generic.HashSet[string]]::new()
$connections = [System.Collections.Generic.HashSet[string]]::new()
$gaps = [System.Collections.Generic.List[double]]::new()
$count = 0; $failures = 0; $phaseReadFailures = 0; $completedNormally = $false; $maxProcesses = 0; $maxTcp = 0; $maxLoopback = 0; $maxNonLoopback = 0; $maxEstablished = 0; $maxUdp = 0
$totalTcpRows = 0L; $totalUdpRows = 0L
$firstSeen = $null; $lastSeen = $null; $lastSample = $null; $nextDue = 0.0; $phase = 'before-launch'
$started = [DateTime]::UtcNow
$ready = @{ ready = $true; observerPid = $PID; startedUtc = $started.ToString('o'); expectedExecutable = $expectedPath; executableSha256 = $executableSha256; intervalMilliseconds = $IntervalMilliseconds } | ConvertTo-Json
[System.IO.File]::WriteAllText((Join-Path $outputPath 'ready.json'), $ready, [System.Text.UTF8Encoding]::new($false))
try {
    while ($clock.Elapsed.TotalSeconds -lt $DurationSeconds -and -not [System.IO.File]::Exists($stopPath)) {
        $sampleStart = $clock.Elapsed.TotalMilliseconds
        if ($null -ne $lastSample) { $gaps.Add($sampleStart - $lastSample) }
        $lastSample = $sampleStart
        if ([System.IO.File]::Exists($phasePath)) {
            try {
                $candidate = Read-PhaseMarker $phasePath
                if ($candidate -match '^[a-zA-Z0-9-]{1,40}$') { $phase = $candidate }
            } catch {
                # A phase writer can briefly own the file exclusively. Keep
                # sampling with the previous label and retry next iteration.
                $phaseReadFailures++
            }
        }
        try {
            $processes = [OxboxEndpointObserver]::Processes()
            $owned = [System.Collections.Generic.HashSet[int]]::new()
            foreach ($process in $processes) {
                $processId = $process.Id
                if ($known.ContainsKey($processId) -and [OxboxEndpointObserver]::Started($processId) -eq $known[$processId]) { [void]$owned.Add($processId) }
                if ($process.Name -ieq [System.IO.Path]::GetFileName($expectedPath)) {
                    $path = [OxboxEndpointObserver]::Path($processId)
                    if ($path -and [System.IO.Path]::GetFullPath($path).Equals($expectedPath, [StringComparison]::OrdinalIgnoreCase)) {
                        $created = [OxboxEndpointObserver]::Started($processId)
                        if ($created -gt 0) { $known[$processId] = $created; [void]$owned.Add($processId); [void]$rootIds.Add($processId) }
                    }
                }
            }
            do {
                $added = $false
                foreach ($process in $processes) {
                    if (-not $owned.Contains($process.Id) -and $owned.Contains($process.Parent)) {
                        $created = [OxboxEndpointObserver]::Started($process.Id)
                        if ($created -ge $known[$process.Parent] -and $created -gt 0) {
                            $known[$process.Id] = $created; [void]$owned.Add($process.Id); $added = $true
                        }
                    }
                }
            } while ($added)
            foreach ($process in $processes) { if ($owned.Contains($process.Id)) { [void]$names.Add($process.Name) } }
            $tcp = 0; $established = 0; $loopback = 0; $nonLoopback = 0; $public = 0; $udp = 0
            $states = @{}
            foreach ($endpoint in [OxboxEndpointObserver]::Endpoints()) {
                if (-not $owned.Contains($endpoint.Pid)) { continue }
                if (-not $endpoint.Tcp) { $udp++; continue }
                $tcp++; $stateKey = [string]$endpoint.State
                $states[$stateKey] = 1 + [int]$states[$stateKey]
                if ($endpoint.Loopback) { $loopback++ }
                elseif (-not $endpoint.NoRemote) {
                    $nonLoopback++
                    if (-not $endpoint.Private) { $public++ }
                    if ($endpoint.State -eq 5) { $established++; [void]$connections.Add($endpoint.Key) }
                }
            }
            if ($owned.Count -gt 0) { if ($null -eq $firstSeen) { $firstSeen = $sampleStart }; $lastSeen = $sampleStart }
            $maxProcesses = [Math]::Max($maxProcesses, $owned.Count); $maxNonLoopback = [Math]::Max($maxNonLoopback, $nonLoopback)
            $maxTcp = [Math]::Max($maxTcp, $tcp); $maxLoopback = [Math]::Max($maxLoopback, $loopback)
            $totalTcpRows += $tcp; $totalUdpRows += $udp
            $maxEstablished = [Math]::Max($maxEstablished, $established); $maxUdp = [Math]::Max($maxUdp, $udp)
            @{ elapsedMs = [Math]::Round($sampleStart, 3); phase = $phase; processCount = $owned.Count; tcpRows = $tcp; tcpLoopbackRows = $loopback; tcpNonLoopbackRows = $nonLoopback; tcpNonLoopbackEstablished = $established; tcpPublicRows = $public; udpBoundEndpoints = $udp; tcpStateCounts = $states } | ConvertTo-Json -Compress -Depth 4 | ForEach-Object { $writer.WriteLine($_) }
        } catch {
            $failures++
            # Exception types/counts are safe; messages may contain machine paths.
            @{ elapsedMs = [Math]::Round($sampleStart,3); phase=$phase; sampleFailureType=$_.Exception.GetType().Name } | ConvertTo-Json -Compress | ForEach-Object { $writer.WriteLine($_) }
        }
        $count++
        if (($count % 20) -eq 0) { $writer.Flush() }
        $nextDue += $IntervalMilliseconds
        $remaining = $nextDue - $clock.Elapsed.TotalMilliseconds
        if ($remaining -gt 0) { Start-Sleep -Milliseconds ([Math]::Max(1, [int]$remaining)) }
        elseif ($remaining -lt -$IntervalMilliseconds) { $nextDue = $clock.Elapsed.TotalMilliseconds }
    }
    $completedNormally = $true
} finally {
    $writer.Dispose(); $clock.Stop()
    $hashUnchanged = $false
    try { $hashUnchanged = (Read-Sha256 $expectedPath).Equals($executableSha256, [StringComparison]::OrdinalIgnoreCase) } catch { }
    $sortedGaps = @($gaps | Sort-Object)
    $p95 = if ($sortedGaps.Count -gt 0) { $sortedGaps[[Math]::Min($sortedGaps.Count - 1, [int][Math]::Floor($sortedGaps.Count * 0.95))] } else { $null }
    $summary = @{
        executablePath=$expectedPath; executableSha256=$executableSha256
        startedUtc=$started.ToString('o'); finishedUtc=[DateTime]::UtcNow.ToString('o'); requestedIntervalMs=$IntervalMilliseconds
        durationSeconds=[Math]::Round($clock.Elapsed.TotalSeconds,3); sampleCount=$count; sampleFailures=$failures
        maximumGapMs=if($sortedGaps.Count){$sortedGaps[-1]}else{$null}; p95GapMs=$p95
        completedNormally=$completedNormally; transientPhaseReadFailures=$phaseReadFailures
        targetObserved=($null -ne $firstSeen); executableHashUnchanged=$hashUnchanged
        validObservation=(Test-ObservationValidity -CompletedNormally $completedNormally -SampleFailures $failures -MarkerFailures $phaseReadFailures -TargetObserved ($null -ne $firstSeen) -HashUnchanged $hashUnchanged)
        rootPids=@($rootIds | ForEach-Object { $_ }); observedProcessNames=@($names | ForEach-Object { $_ }); maximumProcesses=$maxProcesses
        processesAtLastSuccessfulSample=if($null -ne $owned){$owned.Count}else{0}
        firstOwnedSeenMs=$firstSeen; lastOwnedSeenMs=$lastSeen; maximumTcpRows=$maxTcp; maximumTcpLoopbackRows=$maxLoopback
        sumTcpRows=$totalTcpRows; sumUdpBoundEndpoints=$totalUdpRows; maximumTcpNonLoopbackRows=$maxNonLoopback
        maximumEstablishedNonLoopback=$maxEstablished; distinctEstablishedNonLoopbackEndpoints=$connections.Count; maximumUdpBoundEndpoints=$maxUdp
        limitations=@('Snapshots can miss short-lived sockets, processes, and failed/completed traffic between samples.','UDP tables expose local bindings, not destinations or transmitted packets.','DNS Client service, OS brokers and vendor/system services are not descendants; their traffic is outside PID-owned table coverage.','No packet capture or DNS/ETW tracing was performed. No addresses, command lines or ports were written.','Zero observed endpoints is bounded runtime evidence, not proof of zero network activity.')
    }
    [System.IO.File]::WriteAllText((Join-Path $outputPath 'summary.json'), ($summary | ConvertTo-Json -Depth 5), [System.Text.UTF8Encoding]::new($false))
    $summary | ConvertTo-Json -Depth 5
}
