param(
    [Parameter(Mandatory = $true)][string]$Msi,
    [Parameter(Mandatory = $true)][string]$Dark,
    [Parameter(Mandatory = $true)][string]$OutDirectory
)
$ErrorActionPreference = 'Stop'
# This helper is invoked by verify-native-installers.mjs with bounded, hash-checked
# inputs and a fresh workspace output directory. OpenDatabase mode 0 is read-only.
# dark.exe decompiles/extracts the MSI; it does not invoke Windows Installer actions.
function Read-Table($Database, [string]$Sql, [string[]]$Names) {
    $view = $Database.OpenView($Sql)
    try {
        [void]$view.Execute()
        $rows = @()
        while ($record = $view.Fetch()) {
            try {
                $row = [ordered]@{}
                for ($index = 0; $index -lt $Names.Length; $index++) {
                    $row[$Names[$index]] = $record.StringData($index + 1)
                }
                $rows += [pscustomobject]$row
            } finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($record) }
        }
        return $rows
    } finally {
        [void]$view.Close()
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($view)
    }
}
$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $null
try {
    $database = $installer.OpenDatabase($Msi, 0)
    $report = [ordered]@{
        Properties = @(Read-Table $database 'SELECT `Property`, `Value` FROM `Property`' @('Property', 'Value'))
        Files = @(Read-Table $database 'SELECT `File`, `FileName`, `FileSize` FROM `File`' @('Id', 'Name', 'Bytes'))
        CustomActions = @(Read-Table $database 'SELECT `Action`, `Type`, `Source`, `Target` FROM `CustomAction`' @('Action', 'Type', 'Source', 'Target'))
        Sequence = @(Read-Table $database 'SELECT `Action`, `Condition`, `Sequence` FROM `InstallExecuteSequence`' @('Action', 'Condition', 'Sequence'))
    }
    foreach ($file in $report.Files) { $file.Name = ($file.Name -split '\|')[-1] }
    $json = $report | ConvertTo-Json -Depth 8
    [IO.File]::WriteAllText((Join-Path $OutDirectory 'msi-tables.json'), $json, [Text.UTF8Encoding]::new($false))
} finally {
    if ($null -ne $database) { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($database) }
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($installer)
}
& $Dark '-nologo' '-x' (Join-Path $OutDirectory 'payload') $Msi (Join-Path $OutDirectory 'installer.wxs') 2>&1 |
    Out-File -LiteralPath (Join-Path $OutDirectory 'dark.log') -Encoding utf8
if ($LASTEXITCODE -ne 0) { throw 'WiX dark MSI extraction failed; inspect dark.log.' }
