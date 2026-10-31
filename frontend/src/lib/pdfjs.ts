// Minimal pdf.js fallback viewer for browsers that cannot render PDFs
// inline. Experimental: only activated when the native path is
// unavailable (or forced with ?pdfjs=1, disabled with ?pdfjs=0).
//
// Supports pinch-zoom (1x-5x, double-tap resets) and an optional
// horizontal slideshow mode — one page per view with scroll-snap and a
// page indicator — toggled with the corner button and persisted in
// localStorage.
const ZOOM_MAX = 5
const FLOW_KEY = 'pdfjs-flow'

let pdfjsPromise: Promise<any> | null = null

function loadPdfJs(): Promise<any> {
	pdfjsPromise ??= new Promise((resolve, reject) => {
		const script = document.createElement('script')
		script.src = '/pdf.min.js'
		script.onload = () => resolve((window as any).pdfjsLib)
		script.onerror = () => {
			pdfjsPromise = null
			reject(new Error('failed to load /pdf.min.js'))
		}
		document.head.appendChild(script)
	})
	return pdfjsPromise
}

export function pdfNeedsFallback(): boolean {
	const forced = new URLSearchParams(window.location.search).get('pdfjs')
	if (forced === '1') return true
	if (forced === '0') return false

	// Browsers without any built-in PDF viewer say so explicitly.
	if ((navigator as any).pdfViewerEnabled === false) return true

	// Chromium on Android reports pdfViewerEnabled === true because
	// main-frame navigations render, but it still replaces *embedded*
	// PDFs with a placeholder. Everything Android that is not Firefox
	// (Firefox ships pdf.js and embeds fine) takes the fallback.
	const ua = navigator.userAgent
	if (/Android/i.test(ua) && !/Firefox/i.test(ua)) return true

	return false
}

type PageMeta = {
	n: number
	holder: HTMLElement
	box: HTMLElement
}

export async function mountPdfJsViewer(
	wrapper: HTMLElement,
	url: string,
): Promise<void> {
	const pdfjsLib = await loadPdfJs()
	pdfjsLib.GlobalWorkerOptions.workerSrc = '/pdf.worker.min.js'
	const pdf = await pdfjsLib.getDocument({ url }).promise

	wrapper.classList.add('pdfjs-wrap')

	const viewer = document.createElement('div')
	viewer.className = 'pdfjs-viewer'
	wrapper.appendChild(viewer)

	const indicator = document.createElement('div')
	indicator.className = 'pdfjs-indicator'
	indicator.textContent = `1 / ${pdf.numPages}`
	wrapper.appendChild(indicator)

	const flowBtn = document.createElement('button')
	flowBtn.type = 'button'
	flowBtn.className = 'pdfjs-flow-btn'
	flowBtn.title = 'Toggle horizontal / vertical scrolling'
	wrapper.appendChild(flowBtn)

	const horizontal = () => localStorage.getItem(FLOW_KEY) === 'h'

	// Page placeholders; the inner box keeps the page's aspect ratio so
	// layout is stable before, during and after lazy rendering.
	const pages: PageMeta[] = []
	for (let n = 1; n <= pdf.numPages; n++) {
		const page = await pdf.getPage(n)
		const base = page.getViewport({ scale: 1 })
		const holder = document.createElement('div')
		holder.className = 'pdfjs-page'
		holder.dataset.page = String(n)
		const box = document.createElement('div')
		box.className = 'pdfjs-box'
		box.style.aspectRatio = `${base.width} / ${base.height}`
		holder.appendChild(box)
		viewer.appendChild(holder)
		pages.push({ n, holder, box })
	}

	let zoom = 1
	let current = 1
	let generation = 0

	// Sizing: vertical mode sizes holders by width (fit-width * zoom);
	// horizontal mode fits one page per view by height.
	const applyZoom = () => {
		if (horizontal()) {
			for (const p of pages) {
				p.holder.style.height = viewer.clientHeight * zoom + 'px'
			}
		} else {
			for (const p of pages) {
				p.holder.style.width = viewer.clientWidth * zoom + 'px'
			}
		}
		viewer.classList.toggle('pdfjs-zoomed', zoom > 1)
	}

	// Canvases stretch with their box (width: 100%), so zoom feedback is
	// instant; `settle()` clears them afterwards so they re-render crisp
	// at the new scale. `generation` invalidates in-flight renders.
	const settle = () => {
		generation++
		applyZoom()
		for (const p of pages) {
			p.box.querySelector('canvas')?.remove()
			// Re-observe from scratch: observing an already-observed target
			// is a no-op, so the removal above would never re-trigger a
			// render.
			renderIO.unobserve(p.holder)
			renderIO.observe(p.holder)
		}
	}

	// Lazy rendering: render pages near the viewport; the canvas check
	// keeps this idempotent.
	const renderIO = new IntersectionObserver(
		(entries) => {
			for (const entry of entries) {
				if (!entry.isIntersecting) continue
				const p = pages.find((q) => q.holder === entry.target)
				if (p) renderPage(pdf, p)
			}
		},
		{ root: viewer, rootMargin: '100% 0px' },
	)

	// Page tracking for the indicator: fires when a page crosses the
	// middle band of the viewer. Zoom is per-page, so moving to another
	// page resets it — this is what makes zooming reliable in slideshow
	// mode. The band's orientation depends on the scroll direction, and
	// IntersectionObserver options are immutable — so the observer is
	// rebuilt whenever the mode changes.
	let trackIO: IntersectionObserver | null = null
	const trackPages = () => {
		trackIO?.disconnect()
		trackIO = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					if (!entry.isIntersecting) continue
					const n = Number((entry.target as HTMLElement).dataset.page)
					if (n === current) continue
					current = n
					indicator.textContent = `${current} / ${pdf.numPages}`
					if (zoom > 1) settle()
				}
			},
			{
				root: viewer,
				// Percentages resolve against the root's height for
				// top/bottom and its width for left/right — rotate the
				// middle band to match the scroll direction.
				rootMargin: horizontal()
					? '0px -45% 0px -45%'
					: '-45% 0px -45% 0px',
			},
		)
		for (const p of pages) trackIO!.observe(p.holder)
	}

	async function renderPage(pdf: any, p: PageMeta): Promise<void> {
		if (p.box.querySelector('canvas')) return
		const gen = generation
		const page = await pdf.getPage(p.n)
		if (gen !== generation || p.box.querySelector('canvas')) return
		const base = page.getViewport({ scale: 1 })
		const dpr = window.devicePixelRatio || 1
		const target = p.box.clientWidth || p.holder.clientWidth || viewer.clientWidth * zoom
		const viewport = page.getViewport({
			scale: (target / base.width) * dpr,
		})
		const canvas = document.createElement('canvas')
		canvas.width = Math.floor(viewport.width)
		canvas.height = Math.floor(viewport.height)
		canvas.style.width = '100%'
		canvas.style.height = 'auto'
		p.box.appendChild(canvas)
		page.render({ canvasContext: canvas.getContext('2d'), viewport })
	}

	// Pinch-zoom: with `touch-action: pan-x pan-y` the browser hands the
	// two-finger gesture to us instead of zooming the page, while native
	// one-finger scrolling keeps working (including free pan when zoomed).
	const distance = (t: TouchList) => Math.hypot(t[0].clientX - t[1].clientX, t[0].clientY - t[1].clientY)
	let pinch: { dist: number; zoom: number } | null = null
	viewer.addEventListener(
		'touchstart',
		(e) => {
			if (e.touches.length === 2) pinch = { dist: distance(e.touches), zoom }
		},
		{ passive: true },
	)
	viewer.addEventListener(
		'touchmove',
		(e) => {
			if (!pinch || e.touches.length < 2) return
			e.preventDefault()
			zoom = Math.min(Math.max(pinch.zoom * (distance(e.touches) / pinch.dist), 1), ZOOM_MAX)
			applyZoom()
		},
		{ passive: false },
	)
	viewer.addEventListener(
		'touchend',
		(e) => {
			if (pinch && e.touches.length < 2) {
				pinch = null
				settle()
			}
		},
		{ passive: true },
	)

	// Double-tap / double-click resets the zoom.
	viewer.addEventListener('dblclick', () => {
		zoom = 1
		settle()
	})

	const syncFlow = () => {
		const h = horizontal()
		viewer.classList.toggle('pdfjs-h', h)
		flowBtn.textContent = h ? '↕' : '↔'
		trackPages()
	}
	flowBtn.addEventListener('click', () => {
		localStorage.setItem(FLOW_KEY, horizontal() ? 'v' : 'h')
		syncFlow()
		settle()
		// Keep the current page in view after the mode switch.
		if (horizontal()) {
			viewer.scrollTo({ left: pages[current - 1].holder.offsetLeft, top: 0 })
		} else {
			viewer.scrollTo({ left: 0, top: pages[current - 1].holder.offsetTop })
		}
	})
	syncFlow()

	applyZoom()
	for (const p of pages) {
		renderIO.observe(p.holder)
	}
}
