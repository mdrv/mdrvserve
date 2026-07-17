<script lang='ts'>
	let {
		show,
		source,
		onclose,
	}: {
		show: boolean
		source: string
		onclose: () => void
	} = $props()

	let copied = $state(false)

	function copy() {
		navigator.clipboard.writeText(source).then(() => {
			copied = true
			setTimeout(() => (copied = false), 1500)
		})
	}

	function onkeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose()
	}
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && show && onclose()} />

{#if show}
	<div
		class='source-backdrop'
		role='dialog'
		aria-modal='true'
		aria-label='Source code'
		tabindex='-1'
		{onkeydown}
		onclick={(e) => {
			if (e.target === e.currentTarget) onclose()
		}}
	>
		<div class='source-modal' role='presentation'>
			<div class='source-header'>
				<span>Source</span>
				<div class='source-actions'>
					<button class='source-copy-btn' onclick={copy}>
						{copied ? 'Copied!' : 'Copy'}
					</button>
					<button class='source-close-btn' onclick={onclose} aria-label='Close'>
						✕
					</button>
				</div>
			</div>
			<pre class='source-body'><code>{source}</code></pre>
		</div>
	</div>
{/if}
