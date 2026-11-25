<script lang="ts">
  // An agent seen from afar on the « Workspace » (spec 2026-11-25-vue-workspace-design.md, « De loin ») : a disc with
  // its state, its name, what it does and the gauge of its plan, and on its right flank the satellites that have
  // something to say. It waits for an answer: yellow, with a halo.
  import { t } from '../../lib/i18n';
  import { cardActivity } from '../../lib/sidebar';
  import type { Agent } from '../../lib/types';
  import { DISC, type Disc, type Placed, type SatKind } from '../../lib/workspace-layout';

  let { disc, agent, onpick }: { disc: Placed<Disc>; agent: Agent; onpick: (agentId: string) => void } = $props();

  const state = $derived(t(`workspace.state.${disc.state}`));
  const percent = $derived(disc.total ? Math.round((disc.done / disc.total) * 100) : 0);

  const activity = $derived.by(() => {
    if (disc.state === 'wait') return t('workspace.waiting');
    const act = cardActivity(agent, 0);
    if (act?.kind === 'doing') return act.what ? `${act.verb} ${act.what}` : act.verb;
    if (act?.kind === 'thinking') return t('stats.overview.thinking');
    return '';
  });

  // The gauge: a segment a task, 116 px in all, 3 px apart.
  const GAUGE = 116;
  const seg = $derived(disc.total ? (GAUGE - (disc.total - 1) * 3) / disc.total : 0);

  // The satellites: arcs of 24° between the radii 102 and 142, 4° apart, centred on the right flank.
  const R_IN = 102;
  const R_OUT = 142;
  const SPAN = 24;
  const GAP = 4;
  const pt = (r: number, deg: number) => {
    const a = (deg * Math.PI) / 180;
    return `${(r * Math.cos(a)).toFixed(1)} ${(r * Math.sin(a)).toFixed(1)}`;
  };
  const arcs = $derived.by(() => {
    const n = disc.satellites.length;
    let a = -(n * SPAN + (n - 1) * GAP) / 2;
    return disc.satellites.map((s) => {
      const a1 = a;
      const a2 = a + SPAN;
      a += SPAN + GAP;
      const mid = ((a1 + a2) / 2) * (Math.PI / 180);
      const rc = (R_IN + R_OUT) / 2;
      return {
        ...s,
        d: `M${pt(R_OUT, a1)} A${R_OUT} ${R_OUT} 0 0 1 ${pt(R_OUT, a2)} L${pt(R_IN, a2)} A${R_IN} ${R_IN} 0 0 0 ${pt(R_IN, a1)} Z`,
        cx: rc * Math.cos(mid),
        cy: rc * Math.sin(mid),
      };
    });
  });
  const satWord = (kind: SatKind, count: number) =>
    kind === 'team' || kind === 'question' ? t(`workspace.sat.${kind}`, { count }) : '';

  function key(e: KeyboardEvent) {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    e.preventDefault();
    onpick(disc.agentId);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<g
  class="disc {disc.state}"
  transform="translate({disc.x + disc.w / 2} {disc.y + disc.h / 2})"
  role="button"
  tabindex="0"
  aria-label={t('workspace.discLabel', { name: disc.name, state })}
  onclick={() => onpick(disc.agentId)}
  onkeydown={key}
>
  {#if disc.state === 'wait'}<circle class="halo" r={DISC.r + 6} />{/if}
  <circle class="orbit" class:spin={disc.state === 'run'} r={DISC.orbit} />
  <circle class="core" r={DISC.r} />
  <circle class="dot" cx={-36} cy={-51} r="3.5" />
  <text class="st" x="4" y="-48" text-anchor="middle">{state}</text>
  <text class="name" y="-20" text-anchor="middle">{disc.name}</text>
  <text class="act" y="0" text-anchor="middle">{activity}</text>
  {#if disc.total}
    {#each { length: disc.total } as _, i (i)}
      <rect class="seg" class:on={i < disc.done} x={-GAUGE / 2 + i * (seg + 3)} y="22" width={seg} height="6" rx="3" />
    {/each}
    <text class="pct" y="48" text-anchor="middle"
      >{t('workspace.progress', { done: disc.done, total: disc.total, percent })}</text
    >
  {:else}
    <text class="pct" y="34" text-anchor="middle">{t('workspace.noPlan')}</text>
  {/if}
  {#each arcs as s (s.kind)}
    <path class="sat {s.kind}" d={s.d} />
    <text class="sat-n" x={s.cx} y={s.cy - 1} text-anchor="middle">{s.count}</text>
    <text class="sat-l" x={s.cx} y={s.cy + 10} text-anchor="middle">{satWord(s.kind, s.count)}</text>
  {/each}
</g>

<style>
  .disc {
    --c: var(--idle);
    cursor: pointer;
    outline: none;
  }
  .disc.run {
    --c: var(--run);
  }
  .disc.wait {
    --c: var(--wait);
  }
  .disc.error {
    --c: var(--del);
  }
  .disc.done {
    --c: var(--done);
  }
  .core {
    fill: var(--raise);
    stroke: var(--c);
    stroke-width: 2;
  }
  .wait .core {
    fill: color-mix(in oklch, var(--wait) 9%, var(--raise));
  }
  .disc:focus-visible .core {
    stroke-width: 4;
  }
  .halo {
    fill: none;
    stroke: var(--wait);
    stroke-width: 6;
    opacity: 0.25;
    animation: halo 1.8s ease-in-out infinite;
  }
  .orbit {
    fill: none;
    stroke: var(--line-2);
    stroke-dasharray: 2 7;
  }
  .orbit.spin {
    stroke: var(--run);
    opacity: 0.5;
    animation: orbit 1.4s linear infinite;
  }
  .dot {
    fill: var(--c);
  }
  .st {
    fill: var(--c);
    font: 600 9.5px var(--font);
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  .name {
    fill: var(--text);
    font: 700 17px var(--font);
  }
  .act {
    fill: var(--muted);
    font: 10.5px var(--mono);
  }
  .seg {
    fill: var(--line-2);
  }
  .seg.on {
    fill: var(--run);
  }
  .pct {
    fill: var(--soft);
    font: 500 10.5px var(--mono);
  }
  .sat {
    fill: var(--raise);
    stroke: var(--line-2);
    stroke-width: 1.5;
    stroke-linejoin: round;
  }
  .sat.team {
    stroke: var(--run);
  }
  .sat.question {
    stroke: var(--wait);
    fill: color-mix(in oklch, var(--wait) 22%, var(--raise));
  }
  .sat-n {
    fill: var(--text);
    font: 700 13px var(--font);
  }
  .sat-l {
    fill: var(--muted);
    font: 500 8.5px var(--font);
  }
  @keyframes halo {
    50% {
      opacity: 0.06;
    }
  }
  @keyframes orbit {
    to {
      stroke-dashoffset: -9;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .halo,
    .orbit.spin {
      animation: none;
    }
  }
</style>
