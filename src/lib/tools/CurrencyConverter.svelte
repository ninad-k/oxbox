<script lang="ts">
 import {onMount} from 'svelte';
 import {open} from '@tauri-apps/plugin-dialog';
 import {openUrl} from '@tauri-apps/plugin-opener';
 import {inDesktopApp} from '#lib/ipc.ts';
 import {currencyApi,type CurrencyStatus} from '#lib/currency-api.ts';
 import Notice from '#lib/ui/Notice.svelte';
 import CopyButton from '#lib/ui/CopyButton.svelte';
 let status=$state<CurrencyStatus>({networkEnabled:false,networkIssue:null,book:null}),amount=$state('1'),from=$state('EUR'),to=$state('USD'),result=$state(''),busy=$state(false),error=$state('');let requestId=0,alive=true;
 const codes=$derived(Object.keys(status.book?.rates??{}));
 onMount(()=>{currencyApi.currencyStatus().then(v=>{if(alive)status=v;}).catch(e=>{if(alive)error=String(e);});return()=>{alive=false;requestId++;};});
 async function run(work:()=>Promise<unknown>){if(busy)return;busy=true;error='';try{await work();const next=await currencyApi.currencyStatus();if(alive)status=next;}catch(e){if(alive)error=String(e);}finally{if(alive)busy=false;}}
 async function choose(){try{const path=await open({multiple:false,filters:[{name:'ECB XML',extensions:['xml']}]});if(alive&&typeof path==='string')await run(()=>currencyApi.currencyImport(path));}catch(e){error=String(e);}}
 $effect(()=>{const current=status.book;const value=amount,source=from,target=to;const id=++requestId;result='';if(!current||!value.trim())return;const number=Number(value);if(!Number.isFinite(number)){error='Enter a finite amount.';return;}currencyApi.currencyConvert(number,source,target).then(v=>{if(alive&&id===requestId){result=String(Number(v.toPrecision(12)));error='';}}).catch(e=>{if(alive&&id===requestId)error=String(e);});});
</script>
<div class="tool" aria-busy={busy}>
 <Notice kind="info">Currency reference rates are dated estimates for information. Amounts stay on this computer. Import an ECB XML file to work entirely offline.</Notice>
 <button class="btn" disabled={busy} onclick={choose}>Import ECB rate XML</button>
 <label><input type="checkbox" checked={status.networkEnabled} disabled={busy||!status.networkIssue} onchange={e=>run(()=>currencyApi.currencySetNetwork(e.currentTarget.checked))}/> Allow manual HTTPS refresh from the European Central Bank</label>
 {#if !status.networkIssue}<Notice kind="info">Online refresh awaits the repository's required network-feature issue. Local rate-file import and conversion are available.</Notice>{:else}<p>Network review: <a href={status.networkIssue} target="_blank" rel="noopener noreferrer" onclick={event=>{if(inDesktopApp()){event.preventDefault();void openUrl(status.networkIssue!).catch(cause=>{error=String(cause);});}}}>{status.networkIssue}</a></p>{/if}
 <button class="btn" disabled={busy||!status.networkEnabled} onclick={()=>run(()=>currencyApi.currencyRefresh(true))}>Refresh ECB reference rates</button>
 {#if status.book}<p>Rate date: {status.book.date} · Cached locally. Rates may be stale; verify the date before use.</p><div class="row"><label class="field"><span class="field-label">Amount</span><input class="input" bind:value={amount} inputmode="decimal"/></label><label class="field"><span class="field-label">From</span><select class="input" bind:value={from}>{#each codes as code}<option>{code}</option>{/each}</select></label><button class="btn" onclick={()=>{[from,to]=[to,from];}}>Swap</button><label class="field"><span class="field-label">To</span><select class="input" bind:value={to}>{#each codes as code}<option>{code}</option>{/each}</select></label></div><label class="field"><span class="field-label">Converted amount</span><input class="input mono" readonly value={result}/></label><CopyButton text={result}/>{:else}<Notice kind="info">No rates cached. Import a daily rate table to start.</Notice>{/if}
 {#if error}<Notice kind="error">{error}</Notice>{/if}
</div>
<style>.tool{display:grid;gap:var(--s-4)}.row{display:flex;gap:var(--s-3);align-items:end;flex-wrap:wrap}.field{flex:1}p{color:var(--text-2);font-size:var(--text-sm)}</style>
