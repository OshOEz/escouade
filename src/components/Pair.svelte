<script lang="ts">
  // In a browser not paired yet with its server: the code `escouade-server pair` shows, then the app (loaded again).
  import { t } from '../lib/i18n';
  import { pair } from '../lib/transport/device';

  let { onPaired = () => location.reload() }: { onPaired?: () => void } = $props();

  let code = $state('');
  let name = $state(t('shell.pair.defaultName'));
  let busy = $state(false);
  let error = $state('');

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      if (await pair(code.trim(), name.trim())) return onPaired();
      error = t('shell.pair.refused');
      code = '';
    } catch {
      error = t('shell.pair.failed');
    } finally {
      busy = false;
    }
  }
</script>

<main class="pair">
  <form onsubmit={submit} aria-labelledby="pair-title">
    <h1 id="pair-title">{t('shell.pair.title')}</h1>
    <p>{t('shell.pair.intro')}</p>
    <label>
      {t('shell.pair.code')}
      <input bind:value={code} required autocomplete="off" spellcheck="false" />
    </label>
    <label>
      {t('shell.pair.name')}
      <input bind:value={name} required maxlength="64" />
    </label>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <button class="btn primary" type="submit" disabled={busy}>{t('shell.pair.submit')}</button>
  </form>
</main>

<style>
  .pair {
    display: grid;
    place-items: center;
    min-height: 100vh;
    padding: 16px;
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: min(380px, 100%);
  }
  h1 {
    font-size: 18px;
    margin: 0;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .error {
    color: var(--del);
  }
</style>
