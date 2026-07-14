<script lang='ts'>
	import { onMount } from 'svelte'
	import Sidebar from './lib/Sidebar.svelte'
	import SidebarToggle from './lib/SidebarToggle.svelte'
	import ThemeModal from './lib/ThemeModal.svelte'
	import TopControls from './lib/TopControls.svelte'
	import type { ServerData } from './lib/types'

	let { data }: { data: ServerData } = $props()

	let sidebarCollapsed = $state(
		localStorage.getItem('sidebar-collapsed') === 'true',
	)
	let showThemeModal = $state(false)
	let theme = $state(
		document.documentElement.getAttribute('data-theme') || 'catppuccin-mocha',
	)
	let textZoom = $state(
		parseFloat(localStorage.getItem('text-zoom') || '1') || 1,
	)

	$effect(() => {
		document.documentElement.classList.toggle(
			'sidebar-collapsed',
			sidebarCollapsed,
		)
		localStorage.setItem('sidebar-collapsed', String(sidebarCollapsed))
	})

	$effect(() => {
		document.documentElement.setAttribute('data-theme', theme)
		localStorage.setItem('theme', theme)
	})

	$effect(() => {
		document.documentElement.style.setProperty('--text-zoom', String(textZoom))
		localStorage.setItem('text-zoom', String(textZoom))
	})

	let mermaidApi: any = null
	let mermaidLoading = false

	function mermaidTheme(t: string): string {
		return t === 'catppuccin-latte' ? 'default' : 'dark'
	}

	$effect(() => {
		if (!data.mermaidEnabled || !data.content) return
		const mTheme = mermaidTheme(theme)

		// Transform <pre><code class="language-mermaid"> into <div class="mermaid">
		document.querySelectorAll('pre code.language-mermaid').forEach((codeEl) => {
			const pre = codeEl.parentElement
			if (!pre) return
			const div = document.createElement('div')
			div.className = 'mermaid'
			div.textContent = codeEl.textContent
			div.setAttribute('data-original', codeEl.textContent || '')
			pre.replaceWith(div)
		})

		const runMermaid = () => {
			if (!mermaidApi) return
			mermaidApi.initialize({
				startOnLoad: false,
				theme: mTheme,
				securityLevel: 'loose',
			})
			document.querySelectorAll('.mermaid').forEach((el) => {
				el.removeAttribute('data-processed')
				const original = el.getAttribute('data-original')
				if (original !== null) el.textContent = original
			})
			mermaidApi.run()
		}

		if (mermaidApi) {
			runMermaid()
		} else if (!mermaidLoading && !(window as any).mermaid) {
			mermaidLoading = true
			const script = document.createElement('script')
			script.src = '/mermaid.min.js'
			script.async = true
			script.onload = () => {
				mermaidApi = (window as any).mermaid
				runMermaid()
			}
			document.head.appendChild(script)
		}
	})

	onMount(() => {
		const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
		const url = `${proto}//${window.location.host}/ws`
		let ws: WebSocket
		let timer: ReturnType<typeof setTimeout>
		let closed = false

		function connect() {
			ws = new WebSocket(url)
			ws.onmessage = (e) => {
				try {
					const msg = JSON.parse(e.data)
					if (msg.type === 'Reload') window.location.reload()
				} catch {
				}
			}
			ws.onclose = () => {
				if (closed) return
				timer = setTimeout(connect, 3000)
			}
		}

		connect()

		return () => {
			closed = true
			clearTimeout(timer)
			ws.close()
		}
	})
</script>

<div class={data.showNavigation ? 'has-nav' : 'single-file'}>
	{#if data.showNavigation}
		<SidebarToggle
			collapsed={sidebarCollapsed}
			onclick={() => (sidebarCollapsed = !sidebarCollapsed)}
		/>
		<Sidebar navItems={data.navItems} collapsed={sidebarCollapsed} />
	{/if}

	<TopControls
		bind:textZoom
		{theme}
		onopentheme={() => (showThemeModal = true)}
	/>

	<div id='content'>{@html data.content}</div>

	<ThemeModal
		show={showThemeModal}
		{theme}
		onselect={(t) => (theme = t)}
		onclose={() => (showThemeModal = false)}
	/>
</div>
