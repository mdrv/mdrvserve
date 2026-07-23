import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import type { ServerData } from './lib/types'

const dataEl = document.getElementById('__mdrv-data')
let data: ServerData
try {
	data = JSON.parse(dataEl?.textContent || '{}')
} catch {
	data = {
		content: '<p>Error: invalid data</p>',
		sourceContent: '',
		navItems: [],
		pageTitle: 'mdrvserve',
		showNavigation: false,
		mermaidEnabled: false,
		isTypst: false,
		contentFreeflow: '',
	}
}

// Update <title> from server data
if (data.pageTitle) {
	document.title = data.pageTitle
}

const app = mount(App, {
	target: document.getElementById('app')!,
	props: { data },
})

export default app
