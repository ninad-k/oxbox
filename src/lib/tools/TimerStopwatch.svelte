<script lang="ts">
  import { clocks } from '#lib/clocks.svelte.ts';
  import { formatTime } from '#lib/time.ts';
  import Segmented from '#lib/ui/Segmented.svelte';
  import Notice from '#lib/ui/Notice.svelte';
  let mode=$state('timer'), minutes=$state(clocks.timer.duration / 60000), error=$state('');
  const clock=$derived((clocks.revision, mode==='timer'?clocks.timer:clocks.stopwatch));
  const display=$derived(formatTime(mode==='timer'?clock.remaining(clocks.now):clock.elapsed(clocks.now)));
  async function preset(value:number){if(clocks.busy)return;minutes=value;await clocks.resetTimer(value*60000);}
  function reset(){if(clocks.busy)return;if(mode==='timer'){void clocks.resetTimer();return;}clock.reset();clocks.laps=[];clocks.notice='';clocks.revision++;}
  async function toggle(){
    if(clocks.busy)return;
    if(mode==='timer') {
      if(!Number.isFinite(minutes)||minutes<=0||minutes>10080){error='Choose a duration between 0 and 10080 minutes.';return;}
      error=''; if(!clock.running && (clock.accumulated===0 || clock.completed))clock.duration=minutes*60000; await clocks.toggleTimer();
    } else {if(clock.running)clock.pause(performance.now());else clock.start(performance.now());clocks.revision++;}
  }
  function key(e:KeyboardEvent){if(e.target instanceof HTMLElement && /^(INPUT|TEXTAREA|SELECT|BUTTON)$/.test(e.target.tagName))return;if(e.code==='Space'){e.preventDefault();void toggle();}else if(e.key.toLowerCase()==='r')reset();}
</script>
<svelte:window onkeydown={key}/>
<div class="tool">
  <Segmented label="Clock mode" options={[{value:'timer',label:'Timer'},{value:'stopwatch',label:'Stopwatch'}]} bind:value={mode}/>
  {#if mode==='timer'}<div class="row">{#each [1,5,10,25] as m}<button class="btn" disabled={clocks.busy} onclick={()=>preset(m)}>{m} min</button>{/each}</div>
    <label class="field"><span class="field-label">Custom duration (minutes)</span><input class="input" type="number" min="0.01" max="10080" step="any" bind:value={minutes} disabled={clocks.busy || clock.running || (clock.accumulated > 0 && !clock.completed)}/></label>{/if}
  <output class="display mono" aria-label={mode==='timer'?'Time remaining':'Elapsed time'}>{display}</output>
  <div class="row"><button class="btn btn-primary" onclick={toggle} disabled={clocks.busy}>{clock.running?'Pause':'Start'}</button><button class="btn" onclick={reset} disabled={clocks.busy}>Reset</button>
  {#if mode==='stopwatch'}<button class="btn" disabled={!clock.running || clocks.laps.length>=1000} onclick={()=>clocks.laps=[...clocks.laps,clock.elapsed(performance.now())]}>Lap</button>{/if}</div>
  <p>Space starts or pauses; R resets. Clocks continue while you browse tools or hide the launcher.</p>
  {#if mode==='stopwatch' && clocks.laps.length}<ol>{#each clocks.laps as lap}<li class="mono">{formatTime(lap)}.{String(Math.floor(lap%1000)).padStart(3,'0')}</li>{/each}</ol>{/if}
  {#if error || clocks.error}<Notice kind="error">{error || clocks.error}</Notice>{/if}
  {#if clocks.notice}<Notice kind="info">{clocks.notice}</Notice>{/if}
</div>
<style>.tool{display:grid;gap:var(--s-4);max-width:720px}.row{display:flex;gap:var(--s-2);flex-wrap:wrap}.display{font-size:var(--s-8);font-variant-numeric:tabular-nums}p{color:var(--text-2);font-size:var(--text-sm)}</style>
