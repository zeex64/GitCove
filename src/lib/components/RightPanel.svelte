<script lang="ts">
  import type { RepoStore } from '../state/repo.svelte'
  import ChangesPanel from './ChangesPanel.svelte'
  import CommitPanel from './CommitPanel.svelte'

  let { repo }: { repo: RepoStore } = $props()
</script>

{#if repo.selection.kind === 'commit'}
  <CommitPanel {repo} id={repo.selection.id} />
{:else}
  <ChangesPanel {repo} />
{/if}
