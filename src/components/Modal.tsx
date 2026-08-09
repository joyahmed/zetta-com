import { useEffect } from 'react';

/// Settings and the key list live here rather than stacked under the roster.
///
/// The main screen is for the two things this app is: who you are talking to,
/// and what has been said. Everything else is a place you go and come back
/// from, and a modal says that in a way a disclosure never did.
export const Modal = ({ title, open, onClose, children }: ModalProps) => {
	// Escape closes it. Anything that covers the screen has to be dismissible
	// without hunting for the button.
	useEffect(() => {
		if (!open) return;
		const onKey = (e: KeyboardEvent) => {
			if (e.key === 'Escape') onClose();
		};
		window.addEventListener('keydown', onKey);
		return () => window.removeEventListener('keydown', onKey);
	}, [open, onClose]);

	if (!open) return null;

	return (
		<div
			// Still top-aligned rather than centred, so a panel that grows does
			// it downwards instead of shifting under the pointer. The gap above
			// is only what it takes to keep the header visible behind it.
			className='fixed inset-0 z-50 flex items-start justify-center bg-scrim/60 p-3 pt-10 backdrop-blur-sm'
			onClick={onClose}
		>
			<div
				// Stops a click inside the panel from reaching the backdrop and
				// closing the thing you are trying to use.
				onClick={e => e.stopPropagation()}
				// Sized against the window rather than a flat 80vh: the
				// shortcut list is twenty-odd rows and was scrolling inside a
				// box with empty space below it. This leaves the same gap top
				// and bottom and gives everything between them to the panel.
				// `surface`, not `canvas`. A dialog painted the same colour as
				// the window behind it does not read as something raised over
				// the backdrop — it reads as the app having gone strange.
				className='flex max-h-[calc(100vh-5rem)] w-full max-w-md flex-col overflow-hidden rounded-2xl border border-line bg-surface shadow-2xl'
			>
				<header className='flex items-center justify-between border-b border-line-soft px-4 py-3'>
					<h2 className='text-sm font-semibold'>{title}</h2>
					<button
						type='button'
						onClick={onClose}
						aria-label='Close'
						className='rounded-md px-2 py-1 text-faint transition hover:bg-sunken hover:text-ink'
					>
						✕
					</button>
				</header>
				<div className='flex-1 overflow-y-auto p-4'>{children}</div>
			</div>
		</div>
	);
};
