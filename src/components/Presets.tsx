/// Canned messages. Each one also has a global shortcut, so the usual way to
/// send one is without opening the window at all — these buttons exist for
/// when it is already open, and so the shortcuts are discoverable rather than
/// something you have to read a config file to learn.
export const Presets = ({ presets, onSend, disabled }: PresetsProps) => {
	if (presets.length === 0) return null;

	return (
		<div className='flex flex-wrap gap-2'>
			{presets.map(p => (
				<button
					key={p.label}
					type='button'
					disabled={disabled}
					onClick={() => onSend(p.text)}
					title={p.shortcut || undefined}
					className='flex items-center gap-2 rounded-full border border-slate-200 bg-white px-3 py-1.5 text-sm transition hover:border-teal-500 disabled:opacity-55 dark:border-slate-800 dark:bg-slate-900'
				>
					{p.label}
					{p.shortcut && (
						<span className='font-mono text-[0.65rem] text-slate-400 dark:text-slate-500'>
							{p.shortcut.replace('CommandOrControl', 'Ctrl')}
						</span>
					)}
				</button>
			))}
		</div>
	);
};
