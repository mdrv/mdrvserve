<script lang='ts'>
	let {
		textZoom = $bindable(1),
		typstFlow = $bindable<'paged' | 'freeflow'>('paged'),
		isTypst = false,
		freeflowAvailable = false,
		sourceAvailable = true,
		theme,
		lastModified = 0,
		onopentheme,
		onopensource,
	}: {
		textZoom: number
		typstFlow: 'paged' | 'freeflow'
		isTypst: boolean
		freeflowAvailable: boolean
		sourceAvailable: boolean
		theme: string
		lastModified?: number
		onopentheme: () => void
		onopensource: () => void
	} = $props()

	let pct = $derived(Math.round(textZoom * 100))

	// Click-to-edit zoom: the percentage label swaps for a text input so
	// a precise value (e.g. 200) can be typed, beyond the slider's cap.
	const ZOOM_MIN_PCT = 25
	const ZOOM_MAX_PCT = 500
	let editingZoom = $state(false)
	let zoomDraft = $state('')

	function openZoomInput() {
		zoomDraft = String(pct)
		editingZoom = true
	}

	function commitZoomInput() {
		if (!editingZoom) return
		editingZoom = false
		const parsed = Number.parseFloat(zoomDraft)
		if (Number.isFinite(parsed)) {
			textZoom = Math.min(ZOOM_MAX_PCT, Math.max(ZOOM_MIN_PCT, parsed)) / 100
		}
	}

	function zoomInputKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') commitZoomInput()
		else if (event.key === 'Escape') editingZoom = false
	}

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
			class={['typst-flow-btn', { active: freeflowAvailable && typstFlow === 'freeflow' }]}
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
		{#if isTypst && typstFlow === 'paged'}
			<button
				class='zoom-fit-btn'
				onclick={() => (textZoom = 1)}
				aria-label='Fit page width (100%)'
				title='Fit page width (100%)'
			>
				⤢
			</button>
		{/if}
		<input
			class='zoom-slider'
			type='range'
			min='50'
			max='200'
			step='10'
			bind:value={() => pct, (v) => (textZoom = Number(v) / 100)}
		/>
		{#if editingZoom}
			<input
				class='zoom-input'
				type='text'
				inputmode='decimal'
				bind:value={zoomDraft}
				onblur={commitZoomInput}
				onkeydown={zoomInputKeydown}
				aria-label='Zoom percentage'
			/>
		{:else}
			<button
				class='zoom-value'
				onclick={openZoomInput}
				aria-label='Zoom percentage, click to type a value'
				title='Click to type a zoom value'
			>
				{pct}%
			</button>
		{/if}
	</div>
	{#if sourceAvailable}
		<button class='source-btn' onclick={onopensource} aria-label='View source'>
			📋
		</button>
	{/if}
	<button class='theme-toggle' onclick={onopentheme}>🎨</button>
</div>
