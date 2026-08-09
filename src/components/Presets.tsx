/// Canned messages. Each one also has a global shortcut, so the usual way to
/// send one is without opening the window at all — these buttons exist for
/// when it is already open, and so the shortcuts are discoverable rather than
/// something you have to read a config file to learn.
///
/// One row that scrolls sideways rather than a wrapping block. Wrapped, four
/// presets became two rows and six became three, and every one of them was
/// taken out of the message log directly below.
export const Presets = ({ presets, onSend, disabled }: PresetsProps) => {
	if (presets.length === 0) return null;

	return (
		<div className='no-scrollbar flex gap-2 overflow-x-auto'>
			{presets.map(p => (
				<button
					key={p.label}
					type='button'
					disabled={disabled}
					onClick={() => onSend(p.text)}
					title={
						p.shortcut
							? p.shortcut.replace('CommandOrControl', 'Ctrl')
							: undefined
					}
					className='shrink-0 rounded-full border border-line bg-surface px-3 py-1 text-xs text-muted transition hover:border-accent hover:text-ink disabled:opacity-50'
				>
					{p.label}
				</button>
			))}
		</div>
	);
};
