<script lang="ts">
  // In a browser, « Parcourir… » of « Nouveau projet »: the folders of the server (`fs_browse`), under its user's home,
  // to choose the project's, or to clone a repository into (`fs_clone`). The desktop app keeps the system's dialog.
  import { tick } from 'svelte';
  import { api } from '../../lib/ipc';
  import { t } from '../../lib/i18n';
  import { errorText } from '../../lib/refusal';
  import { tip } from '../../lib/tip';
  import type { Listing } from '../../lib/types';
  import Icon from '../Icon.svelte';

  let { onpick }: { onpick: (path: string) => void } = $props();

  let shown = $state(false);
  let listing = $state<Listing | null>(null);
  let url = $state('');
  let busy = $state(false);
  let error = $state('');
  let where = $state<HTMLElement>();

  /** The listing of `path` (the home when none); a refusal said, the folder shown kept. The keyboard's place, lost
   *  with the list drawn again, put on the folder shown. */
  async function go(path?: string) {
    busy = true;
    error = '';
    try {
      listing = await api.fsBrowse(path);
      await tick();
      where?.focus();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  function toggle() {
    shown = !shown;
    if (shown && !listing) go();
  }

  function pick(path: string) {
    shown = false;
    onpick(path);
  }

  async function clone(e: SubmitEvent) {
    e.preventDefault();
    if (!listing || !url.trim()) return;
    busy = true;
    error = '';
    try {
      pick(await api.fsClone(url.trim(), listing.path));
      url = '';
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

<button class="btn" aria-expanded={shown} onclick={toggle}><Icon name="folder" size={14} />{t('shell.newProject.browse')}</button>
{#if shown}
  <div class="sf" role="group" aria-label={t('shell.newProject.server.title')} aria-busy={busy}>
    <div class="head">
      <button
        class="btn"
        aria-label={t('shell.newProject.server.up')}
        disabled={!listing?.parent || busy}
        onclick={() => go(listing?.parent ?? undefined)}><Icon name="up" size={14} /></button
      >
      <span class="path mono" tabindex="-1" bind:this={where}>{listing?.path ?? ''}</span>
      <button class="btn" disabled={!listing || busy} onclick={() => listing && pick(listing.path)}
        >{t('shell.newProject.server.choose')}</button
      >
    </div>
    {#if error}<p class="err" role="alert">{error}</p>{/if}
    {#if listing}
      <ul>
        {#each listing.entries as f (f.name)}
          <li>
            <!-- aria-disabled, not disabled: an unreadable folder is still reached, its tip shown. -->
            <button
              aria-disabled={!f.dir}
              disabled={busy}
              use:tip={f.dir ? undefined : t('shell.newProject.server.unreadable')}
              onclick={() => f.dir && go(`${listing!.path}/${f.name}`)}
              ><Icon name="folder" size={14} />{f.name}{#if f.git}<span class="git">{t('shell.newProject.server.git')}</span>{/if}</button
            >
          </li>
        {:else}
          <li class="none">{t('shell.newProject.server.empty')}</li>
        {/each}
      </ul>
      {#if listing.truncated}<p class="none">{t('shell.newProject.server.truncated')}</p>{/if}
      <form onsubmit={clone}>
        <input class="field mono" bind:value={url} aria-label={t('shell.newProject.server.cloneUrl')} spellcheck="false" />
        <button class="btn" type="submit" disabled={busy || !url.trim()}
          >{busy && url.trim() ? t('shell.newProject.server.cloning') : t('shell.newProject.server.clone')}</button
        >
      </form>
    {/if}
  </div>
{/if}

<style>
  .sf {
    flex-basis: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    border: 1px solid var(--line);
    border-radius: 6px;
  }
  .head,
  form {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 220px;
    overflow: auto;
  }
  li button {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 4px 6px;
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  li button:hover:not(:disabled, [aria-disabled='true']) {
    background: var(--hover);
  }
  li button:disabled,
  li button[aria-disabled='true'] {
    color: var(--dim);
    cursor: default;
  }
  .git {
    margin-left: auto;
    font-family: var(--mono);
    font-size: 11px;
    color: var(--run);
  }
  .none {
    padding: 4px 6px;
    color: var(--dim);
  }
  .err {
    margin: 0;
    color: var(--del);
    font-size: 12px;
  }
  form input {
    flex: 1;
    height: 30px;
    font-size: 12px;
  }
</style>
