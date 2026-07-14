<script lang='ts'>
	let {
		textZoom = $bindable(1),
		theme,
		onopentheme,
	}: {
		textZoom: number
		theme: string
		onopentheme: () => void
	} = $props()

	let pct = $derived(Math.round(textZoom * 100))
</script>

<div class='top-controls'>
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
	<button class='theme-toggle' onclick={onopentheme}>🎨</button>
</div>
