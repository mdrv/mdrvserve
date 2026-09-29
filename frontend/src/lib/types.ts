export interface NavItem {
	type: 'file' | 'dir'
	name: string
	fullPath?: string
	abbr?: string
	children?: NavItem[]
}

export interface ServerData {
	content: string
	sourceContent: string
	navItems: NavItem[]
	pageTitle: string
	showNavigation: boolean
	mermaidEnabled: boolean
	isTypst: boolean
	contentFreeflow: string
	lastModified: number
}
