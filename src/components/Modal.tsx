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
			className='fixed inset-0 z-50 flex items-end justify-center bg-slate-950/50 p-3 backdrop-blur-sm sm:items-center'
			onClick={onClose}
		>
			<div
				// Stops a click inside the panel from reaching the backdrop and
				// closing the thing you are trying to use.
				onClick={e => e.stopPropagation()}
				className='flex max-h-[85vh] w-full max-w-md flex-col overflow-hidden rounded-2xl border border-slate-200 bg-slate-50 shadow-2xl dark:border-slate-800 dark:bg-slate-950'
			>
				<header className='flex items-center justify-between border-b border-slate-200 px-4 py-3 dark:border-slate-800'>
					<h2 className='text-sm font-semibold'>{title}</h2>
					<button
						type='button'
						onClick={onClose}
						aria-label='Close'
						className='rounded-md px-2 py-1 text-slate-400 transition hover:bg-slate-200 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200'
					>
						✕
					</button>
				</header>
				<div className='flex-1 overflow-y-auto p-4'>{children}</div>
			</div>
		</div>
	);
};
