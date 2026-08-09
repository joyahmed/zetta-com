import React from 'react';

/// Catches a render crash and shows what it was.
///
/// React unmounts the whole tree when a render throws, and in a webview with no
/// devtools that arrives as a blank white window — the least informative failure
/// the app can produce, and indistinguishable from a hang. The rest of this
/// codebase refuses to fail silently; the UI should not be the exception.
///
/// A class because there is still no hook form of componentDidCatch.
export class Crash extends React.Component<
	{ children: React.ReactNode },
	{ error: Error | null }
> {
	state: { error: Error | null } = { error: null };

	static getDerivedStateFromError(error: Error) {
		return { error };
	}

	componentDidCatch(error: Error, info: React.ErrorInfo) {
		// Also to the console, so a dev build keeps the component stack that
		// says which component threw — the message alone rarely names it.
		console.error('[crash]', error, info.componentStack);
	}

	render() {
		const { error } = this.state;
		if (!error) return this.props.children;

		return (
			<div className='flex min-h-screen flex-col gap-3 bg-canvas p-4 text-ink'>
				<h1 className='text-sm font-semibold'>Something broke</h1>
				<p className='text-xs text-muted'>
					The window would otherwise have gone blank. Audio and
					discovery are unaffected — they run in the background, not
					here.
				</p>
				<pre className='overflow-auto rounded-lg border border-danger bg-surface p-3 font-mono text-xs whitespace-pre-wrap text-danger'>
					{error.message}
					{error.stack ? `\n\n${error.stack}` : ''}
				</pre>
				<button
					type='button'
					onClick={() => window.location.reload()}
					className='self-start rounded-lg bg-accent px-4 py-2 text-sm font-medium text-on-accent transition hover:bg-accent-hover'
				>
					Reload
				</button>
			</div>
		);
	}
}
