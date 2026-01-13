// Alpine.js application initialization
import Alpine from 'https://esm.sh/alpinejs@3.14.3';

import authStore from './stores/auth.js';
import schedulesStore from './stores/schedules.js';
import timeline from './components/timeline.js';
import scheduleEditor from './components/editor.js';

// Make Alpine available globally BEFORE registering stores/components
// This ensures store methods can access Alpine.store() at runtime
window.Alpine = Alpine;

// Register stores
Alpine.store('auth', authStore);
Alpine.store('schedules', schedulesStore);

// Register components
Alpine.data('timeline', timeline);
Alpine.data('scheduleEditor', scheduleEditor);

// Initialize auth (load key from localStorage)
Alpine.store('auth').init();

// Start Alpine
Alpine.start();
