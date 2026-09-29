<script lang='ts'>
	let {
		textZoom = $bindable(1),
		typstFlow = $bindable<'paged' | 'freeflow'>('paged'),
		isTypst = false,
		freeflowAvailable = false,
		theme,
		lastModified = 0,
		onopentheme,
		onopensource,
	}: {
		textZoom: number
		typstFlow: 'paged' | 'freeflow'
		isTypst: boolean
		freeflowAvailable: boolean
		theme: string
		lastModified?: number
		onopentheme: () => void
		onopensource: () => void
	} = $props()

	let pct = $derived(Math.round(textZoom * 100))

	let now = $state(Date.now())

	$effect(() => {
		const timer = setInterval(() => (now = Date.now()), 30_000)
		return () => clearInterval(timer)
	})

	function relativeTime(ms: number): string {
		if (!ms) return ''
		const secs = Math.max(0, Math.round((now - ms) / 1000))
		if (secs < 10) return 'just now'
		if (secs < 60) return `${secs}s ago`
		const mins = Math.round(secs / 60)
		if (mins < 60) return `${mins}m ago`
		const hours = Math.round(mins / 60)
		if (hours < 24) return `${hours}h ago`
		return new Date(ms).toLocaleDateString()
	}
</script>

<div class='top-controls'>
	{#if lastModified > 0}
		<span class='last-modified' title={new Date(lastModified).toLocaleString()}>
			{relativeTime(lastModified)}
		</span>
	{/if}
	{#if isTypst}
		<button
			class='typst-flow-btn'
			class:active={freeflowAvailable && typstFlow === 'freeflow'}
			disabled={!freeflowAvailable}
			onclick={() => (typstFlow = typstFlow === 'freeflow' ? 'paged' : 'freeflow')}
			aria-label='Toggle Typst free-flow layout'
			title={freeflowAvailable
			? `Layout: ${typstFlow}`
			: "`typst` doesn't have HTML feature enabled"}
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
