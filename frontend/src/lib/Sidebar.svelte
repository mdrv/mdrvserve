<script lang='ts'>
	import SidebarList from './SidebarList.svelte'
	import type { NavItem } from './types'

	let {
		navItems,
		collapsed,
	}: {
		navItems: NavItem[]
		collapsed: boolean
	} = $props()
</script>

<nav class='sidebar' data-collapsed={collapsed}>
	<div class='sidebar-header'></div>
	<div class='sidebar-content'>
		<SidebarList items={navItems} />
	</div>
</nav>
