import { Moon } from './Moon';

/// Shown on the main screen for as long as do-not-disturb is on.
///
/// The tray tick and the lit moon are easy to stop seeing, and a DND you have
/// forgotten about is how a spoken instruction goes unheard with nobody aware
/// it was missed. So it is said in words, in the one place you look, and the
/// whole bar is the way out.
export const DndOn = ({ onOff }: { onOff: () => void }) => (
	<button
		type='button'
		onClick={onOff}
		className='flex w-full items-center gap-2.5 rounded-xl border border-accent bg-accent-soft px-3.5 py-2.5 text-left text-sm text-ink transition hover:border-accent-hover'
	>
		<span className='text-accent'>
			<Moon {...{ size: 16, filled: true }} />
		</span>
		<span className='min-w-0 flex-1'>
			<span className='font-medium'>Do not disturb is on.</span>{' '}
			<span className='text-muted'>No voice or sounds; messages still arrive.</span>
		</span>
		<span className='shrink-0 text-xs font-medium text-accent'>Turn off</span>
	</button>
);

/// Under the talk bar when whoever you are aimed at is on do-not-disturb.
///
/// A note, not a block: sending to a busy person is allowed, and their text
/// still arrives. What it saves is talking for half a minute into a machine
/// that is not playing it.
export const BusyNote = ({ who, many }: { who: string; many: boolean }) => (
	<p className='-mt-1.5 flex items-center gap-1.5 px-1 text-xs text-muted'>
		<Moon {...{ size: 12 }} />
		<span className='min-w-0'>
			{many ? (
				<>
					<span className='text-ink'>{who}</span> are busy — they will see
					messages, not hear voice.
				</>
			) : (
				<>
					<span className='text-ink'>{who}</span> is busy — they will see
					messages, not hear voice.
				</>
			)}
		</span>
	</p>
);
