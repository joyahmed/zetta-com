import { Field } from './Field';

/// Start with Windows, and how long to wait before doing anything about it.
///
/// Off by default. Adding yourself to somebody's login without being asked is
/// the kind of thing a tool does once before it is uninstalled — and this one is
/// installed per-machine, so the entry it writes is per-user and has to be each
/// user's own answer anyway.
export const Startup = ({
	on,
	onChoose,
	delay,
	onDelay,
	onCommit
}: StartupProps) => (
	<div className='flex flex-col gap-1.5'>
		<label className='flex cursor-pointer items-center gap-2.5'>
			<input
				type='checkbox'
				checked={on}
				onChange={e => onChoose(e.currentTarget.checked)}
				className='size-4 shrink-0 accent-accent'
			/>
			<span className='text-sm text-ink'>Start with Windows</span>
		</label>

		{/* Only when it can matter. A wait nothing waits for is a control that
		    invites you to tune something that is not happening. */}
		{on && (
			<Field
				{...{
					label: 'Wait first (seconds)',
					value: delay,
					onChange: onDelay,
					onBlur: onCommit,
					className: 'w-32'
				}}
			/>
		)}

		<p className='text-xs text-muted'>
			{on
				? 'Comes up in the tray, listening, without opening the window. The wait is counted from the launch, not from the power button: Windows runs login items while WiFi is still associating, and a PC that starts looking too early finds nobody and says nothing about it. Set it to 0 on a wired PC.'
				: 'Nobody can reach a PC where the intercom was never launched, and the silence looks the same as being ignored.'}
		</p>
	</div>
);
