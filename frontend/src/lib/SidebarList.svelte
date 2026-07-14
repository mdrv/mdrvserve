<script lang='ts'>
	import Self from './SidebarList.svelte'
	import type { NavItem } from './types'

	let { items }: { items: NavItem[] } = $props()

	function currentPath(): string {
		return window.location.pathname.replace(/^\//, '')
	}
</script>

<ul class='file-list'>
	{#each items as item (item.fullPath ?? item.name)}
		{#if item.type === 'dir'}
			<li class='nav-dir'>
				<span class='nav-dir-name'>{item.name}</span>
				{#if item.children}
					<Self items={item.children} />
				{/if}
			</li>
		{:else}
			<li>
				<a
					href={'/' + (item.fullPath ?? '')}
					data-abbr={item.abbr}
					class:active={item.fullPath === currentPath()}
				>{item.name}</a>
			</li>
		{/if}
	{/each}
</ul>
