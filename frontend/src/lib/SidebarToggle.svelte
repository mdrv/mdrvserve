<script lang='ts'>
	let {
		collapsed,
		onclick,
	}: {
		collapsed: boolean
		onclick: () => void
	} = $props()
</script>

<button
	class='sidebar-toggle'
	{onclick}
	aria-label='Toggle sidebar'
	aria-expanded={!collapsed}
>
	<svg
		width='24'
		height='24'
		viewBox='0 0 24 24'
		fill='none'
		stroke='currentColor'
		stroke-width='1.5'
		stroke-linecap='round'
		stroke-linejoin='round'
	>
		<rect x='3' y='3' width='18' height='18' rx='2' ry='2'></rect>
		<line x1='9' y1='3' x2='9' y2='21'></line>
	</svg>
</button>
