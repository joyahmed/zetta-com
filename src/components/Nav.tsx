import { getCurrentWindow } from '@tauri-apps/api/window';
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
		// p-1.5, not p-2. Four icons plus a button plus the window controls share
		// this bar with the machine name, and the name is the only part of it
		// that has a length nobody chose — 8px per icon is the difference between
		// "Admins-Mac-Studio" reading in full and reading as "Admins-Mac-Stu…".
		className='rounded-lg p-1.5 text-muted transition hover:bg-sunken hover:text-ink'
	>
		{children}
	</button>
);

/// Minimise and close, since the window has no decorations any more.
const WindowButton = ({
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
		className='rounded-md px-2 py-1 text-faint transition hover:bg-sunken hover:text-ink'
	>
		{children}
	</button>
);

/// The top bar: who you are, whether you are on, and the way to everything
/// that is not the main job.
///
/// It is also the title bar. The window is undecorated, so the name, the icon,
/// the drag handle and the window buttons are all this component's problem now.
export const Nav = ({
	me,
	running,
	onToggle,
	onAddPc,
	onShortcuts,
	onSettings,
	onDiagnostics
}: NavProps) => (
	<header className='sticky top-0 z-10 border-b border-line bg-canvas/90 backdrop-blur'>
		{/* Tauri starts a drag when the element actually clicked carries the
		    attribute — it does not inherit — so every non-interactive part of
		    the bar has to say so itself. Miss one and it becomes a dead patch
		    the window cannot be moved by, which is what most of this bar was. */}
		<div
			data-tauri-drag-region
			className='flex items-center justify-end gap-1 px-2 pt-1.5'
		>
			<WindowButton
				{...{
					label: 'Minimise',
					onClick: () => getCurrentWindow().minimize()
				}}
			>
				<svg width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2' strokeLinecap='round'>
					<path d='M5 12h14' />
				</svg>
			</WindowButton>
			{/* close(), not hide(): Rust already intercepts CloseRequested and
			    hides the window instead of quitting, so going through the real
			    close keeps one rule in one place. Quit stays in the tray. */}
			<WindowButton
				{...{
					label: 'Close to tray',
					onClick: () => getCurrentWindow().close()
				}}
			>
				<svg width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2' strokeLinecap='round'>
					<path d='M6 6l12 12M18 6L6 18' />
				</svg>
			</WindowButton>
		</div>

		<div
			data-tauri-drag-region
			className='mx-auto flex w-full max-w-md items-center gap-2 px-4 pt-1 pb-3'
		>
			<img
				data-tauri-drag-region
				src='/icon.png'
				alt=''
				width='24'
				height='24'
				className='shrink-0 rounded'
			/>

			{/* Not a button any more. It was the way into the diagnostics, which
			    made the one obvious thing to grab the window by the one thing
			    you could not — and nothing about a title says "click me". The
			    diagnostics have their own icon beside the others now. */}
			<div data-tauri-drag-region className='min-w-0 flex-1'>
				<h1
					data-tauri-drag-region
					className='truncate text-sm font-semibold tracking-tight'
				>
					Zetta Com
				</h1>
				{/* title as well as truncate. A hostname is not something the
				    user picked to fit here, and a name cut to "Admins-Mac-Stu…"
				    with no way to read the rest is the bar lying about which
				    machine you are sitting at. The dot is outside the truncating
				    span so it survives however long the name is. */}
				<p
					data-tauri-drag-region
					title={me}
					className='flex items-center gap-1.5 text-xs text-muted'
				>
					<Dot {...{ on: running }} />
					<span className='truncate'>{me ? me : ' '}</span>
				</p>
			</div>

			<IconButton {...{ label: 'Add a PC', onClick: onAddPc }}>
				<svg width='18' height='18' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2' strokeLinecap='round'>
					<path d='M12 5v14M5 12h14' />
				</svg>
			</IconButton>
			<IconButton {...{ label: 'Shortcuts', onClick: onShortcuts }}>
				<svg width='18' height='18' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2' strokeLinecap='round'>
					<rect x='2' y='6' width='20' height='12' rx='2' />
					<path d='M6 10h.01M10 10h.01M14 10h.01M18 10h.01M8 14h8' />
				</svg>
			</IconButton>
			{/* A trace, because that is what the panel behind it is: the counters
			    moving or not moving is the whole answer. */}
			<IconButton {...{ label: 'Diagnostics', onClick: onDiagnostics }}>
				<svg width='18' height='18' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2' strokeLinecap='round' strokeLinejoin='round'>
					<path d='M3 12h3.5l2-6 4 13 2.5-7H21' />
				</svg>
			</IconButton>
			{/* 16px, not 18: a gear is eight teeth and a hub where the others are
			    two or three strokes, so at the same nominal size it carries far
			    more ink and sits heavier than everything beside it.
			    The path is also redrawn. The old one was this gear with its arc
			    radii flattened to 1.7 and several sweep flags wrong, so the teeth
			    curled the wrong way and it rendered as a blot — that, not the
			    size, is what made it look odd. */}
			<IconButton {...{ label: 'Settings', onClick: onSettings }}>
				<svg width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2' strokeLinecap='round' strokeLinejoin='round'>
					<circle cx='12' cy='12' r='3' />
					<path d='M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z' />
				</svg>
			</IconButton>

			<button
				type='button'
				onClick={onToggle}
				// The colour is set per state, not once in the base with an
				// override after it — two text-colour utilities on one element
				// are resolved by stylesheet order, which is not something the
				// author of the element gets to decide.
				className={`ml-1 rounded-lg px-3 py-1.5 text-sm font-medium transition ${
					running
						? 'bg-sunken text-ink hover:bg-line'
						: 'bg-accent text-on-accent hover:bg-accent-hover'
				}`}
			>
				{running ? 'Stop' : 'Start'}
			</button>
		</div>
	</header>
);
