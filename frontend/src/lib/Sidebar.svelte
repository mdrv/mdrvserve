<script lang='ts'>
	import { onMount } from 'svelte'
	import SidebarList from './SidebarList.svelte'
	import type { NavItem } from './types'

	let {
		navItems,
		collapsed,
		filter = '',
	}: {
		navItems: NavItem[]
		collapsed: boolean
		filter?: string
	} = $props()

	let navEl: HTMLElement | undefined

	onMount(() => {
		if (!navEl) return
		const saved = localStorage.getItem('sidebar-scroll-top')
		if (saved) navEl.scrollTop = parseInt(saved, 10)

		const save = () => {
			if (navEl) {
				localStorage.setItem('sidebar-scroll-top', String(navEl.scrollTop))
			}
		}
		navEl.addEventListener('scroll', save)
		window.addEventListener('pagehide', save)

		return () => {
			navEl?.removeEventListener('scroll', save)
			window.removeEventListener('pagehide', save)
		}
	})
</script>

<nav class='sidebar' data-collapsed={collapsed} bind:this={navEl}>
	<div class='sidebar-header'></div>
	<div class='sidebar-content'>
		<SidebarList items={navItems} {filter} />
	</div>
</nav>
