import { Dot } from './Dot';

const IconButton = ({
	label,
	onClick,
	children
}: {
	label: string;
	onClick: () => void;
	children: React.ReactNode;
}) => (
	<button
		type='button'
		onClick={onClick}
		title={label}
		aria-label={label}
		className='rounded-lg p-2 text-slate-500 transition hover:bg-slate-200 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100'
	>
		{children}
	</button>
);

/// The top bar: who you are, whether you are on, and the way to everything
/// that is not the main job.
///
/// Settings, the key list and adding a PC all used to be expanders stacked
/// under the roster, which made the screen a pile of everything. They are
/// places you visit now, and the bar is how you get there.
export const Nav = ({
	me,
	running,
	onToggle,
	onAddPc,
	onShortcuts,
	onSettings
}: NavProps) => (
	<header className='sticky top-0 z-10 flex items-center gap-3 border-b border-slate-200 bg-slate-50/90 px-4 py-3 backdrop-blur dark:border-slate-800 dark:bg-slate-950/90'>
		<div className='min-w-0 flex-1'>
			<h1 className='truncate text-sm font-semibold tracking-tight'>
				Zetta Com
			</h1>
			<p className='flex items-center gap-1.5 truncate text-xs text-slate-500 dark:text-slate-400'>
				<Dot on={running} />
				{me ? me : ' '}
			</p>
		</div>

		<IconButton label='Add a PC' onClick={onAddPc}>
			<svg width='18' height='18' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2' strokeLinecap='round'>
				<path d='M12 5v14M5 12h14' />
			</svg>
		</IconButton>
		<IconButton label='Shortcuts' onClick={onShortcuts}>
			<svg width='18' height='18' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2' strokeLinecap='round'>
				<rect x='2' y='6' width='20' height='12' rx='2' />
				<path d='M6 10h.01M10 10h.01M14 10h.01M18 10h.01M8 14h8' />
			</svg>
		</IconButton>
		<IconButton label='Settings' onClick={onSettings}>
			<svg width='18' height='18' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2' strokeLinecap='round'>
				<circle cx='12' cy='12' r='3' />
				<path d='M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-2.9 1.2V21a2 2 0 1 1-4 0v-.1A1.7 1.7 0 0 0 7 19.4a1.7 1.7 0 0 0-1.9.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0-1.2-2.9H1a2 2 0 1 1 0-4h.1A1.7 1.7 0 0 0 2.6 7a1.7 1.7 0 0 0-.3-1.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.9.3H7a1.7 1.7 0 0 0 1-1.5V1a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.9-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.9V7a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z' />
			</svg>
		</IconButton>

		<button
			type='button'
			onClick={onToggle}
			className={`ml-1 rounded-lg px-3 py-1.5 text-sm font-medium text-white transition ${
				running
					? 'bg-slate-700 hover:bg-slate-600'
					: 'bg-teal-600 hover:bg-teal-500'
			}`}
		>
			{running ? 'Stop' : 'Start'}
		</button>
	</header>
);
