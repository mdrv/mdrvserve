<script lang='ts'>
	import Self from './SidebarList.svelte'
	import type { NavItem } from './types'

	let { items, filter = '' }: { items: NavItem[]; filter?: string } = $props()

	function currentPath(): string {
		return window.location.pathname.replace(/^\//, '')
	}

	function globToRegex(pattern: string): RegExp | null {
		const trimmed = pattern.trim()
		if (!trimmed) return null
		const escaped = trimmed.replace(/[.+^${}()|[\]\\]/g, '\\$&')
		const glob = escaped.replace(/\*/g, '.*').replace(/\?/g, '.')
		try {
			return new RegExp(glob, 'i')
		} catch {
			return null
		}
	}

	function filterItems(items: NavItem[], re: RegExp): NavItem[] {
		const result: NavItem[] = []
		for (const item of items) {
			if (item.type === 'file') {
				if (re.test(item.name) || re.test(item.fullPath ?? '')) {
					result.push(item)
				}
			} else if (item.children) {
				const kids = filterItems(item.children, re)
				if (kids.length > 0) result.push({ ...item, children: kids })
			}
		}
		return result
	}

	const filtered = $derived(
		(() => {
			const re = globToRegex(filter)
			return re ? filterItems(items, re) : items
		})(),
	)
</script>

<ul class='file-list'>
	{#each filtered as item (item.fullPath ?? item.name)}
		{#if item.type === 'dir'}
			<li class='nav-dir'>
				<span class='nav-dir-name'>{item.name}</span>
				{#if item.children}
					<Self items={item.children} filter={filter} />
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
