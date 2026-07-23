<script lang='ts'>
	let {
		textZoom = $bindable(1),
		typstFlow = $bindable<'paged' | 'freeflow'>('paged'),
		isTypst = false,
		theme,
		onopentheme,
		onopensource,
	}: {
		textZoom: number
		typstFlow: 'paged' | 'freeflow'
		isTypst: boolean
		theme: string
		onopentheme: () => void
		onopensource: () => void
	} = $props()

	let pct = $derived(Math.round(textZoom * 100))
</script>

<div class='top-controls'>
	{#if isTypst}
		<button
			class='typst-flow-btn'
			class:active={typstFlow === 'freeflow'}
			onclick={() => (typstFlow = typstFlow === 'freeflow' ? 'paged' : 'freeflow')}
			aria-label='Toggle Typst free-flow layout'
			title='Layout: {typstFlow}'
		>
			{typstFlow === 'freeflow' ? '📜' : '📄'}
		</button>
	{/if}
	<div class='zoom-control'>
		<input
			class='zoom-slider'
			type='range'
			min='50'
			max='150'
			step='10'
			bind:value={() => pct, (v) => (textZoom = Number(v) / 100)}
		/>
		<span class='zoom-value'>{pct}%</span>
	</div>
	<button class='source-btn' onclick={onopensource} aria-label='View source'>
		📋
	</button>
	<button class='theme-toggle' onclick={onopentheme}>🎨</button>
</div>
