<script lang='ts'>
	let {
		show,
		theme,
		onselect,
		onclose,
	}: {
		show: boolean
		theme: string
		onselect: (t: string) => void
		onclose: () => void
	} = $props()

	const themes = [
		{
			id: 'catppuccin-latte',
			icon: '☕',
			name: 'Catppuccin Latte',
			swatches: ['#eff1f5', '#4c4f69', '#1e66f5'],
			sample: 'Warm light theme',
		},
		{
			id: 'catppuccin-macchiato',
			icon: '🥛',
			name: 'Catppuccin Macchiato',
			swatches: ['#24273a', '#cad3f5', '#8aadf4'],
			sample: 'Medium contrast',
		},
		{
			id: 'catppuccin-mocha',
			icon: '🐱',
			name: 'Catppuccin Mocha',
			swatches: ['#1e1e2e', '#cdd6f4', '#89b4fa'],
			sample: 'Dark and cozy',
		},
		{
			id: 'light',
			icon: '☀️',
			name: 'Light',
			swatches: ['#fff', '#333', '#0366d6'],
			sample: 'Classic bright',
		},
		{
			id: 'dark',
			icon: '🌙',
			name: 'Dark',
			swatches: ['#0d1117', '#e6edf3', '#58a6ff'],
			sample: 'Classic dark',
		},
	]

	function onBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose()
	}

	function onKeydown(e: KeyboardEvent) {
		if (show && e.key === 'Escape') onclose()
	}

	function onDialogKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose()
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div
	class='theme-modal'
	class:show
	role='dialog'
	aria-modal='true'
	aria-label='Choose theme'
	tabindex='-1'
	onclick={onBackdropClick}
	onkeydown={onDialogKeydown}
>
	<div class='theme-modal-content'>
		<h3>Choose Theme</h3>
		<div class='theme-grid'>
			{#each themes as t (t.id)}
				<button
					type='button'
					class='theme-card'
					data-theme={t.id}
					class:selected={theme === t.id}
					onclick={() => onselect(t.id)}
				>
					<div class='theme-card-icon'>{t.icon}</div>
					<div class='theme-card-name'>{t.name}</div>
					<div class='theme-card-preview'>
						{#each t.swatches as color (color)}
							<span
								class='theme-color-swatch'
								style:background-color={color}
							></span>
						{/each}
					</div>
					<div class='theme-card-sample'>{t.sample}</div>
				</button>
			{/each}
		</div>
	</div>
</div>
