import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { Crash } from './components/Crash';
import './index.css';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
	<React.StrictMode>
		<Crash>
			<App />
		</Crash>
	</React.StrictMode>
);
