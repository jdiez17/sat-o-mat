// Alpine.js application initialization
import Alpine from 'https://esm.sh/alpinejs@3.14.3';

import authStore from './stores/auth.js';
import schedulesStore from './stores/schedules.js';
import trackerStore from './stores/tracker.js';
import timeline from './components/timeline.js';
import scheduleModal from './components/modal.js';

// Make Alpine available globally BEFORE registering stores/components
// This ensures store methods can access Alpine.store() at runtime
window.Alpine = Alpine;

// Register stores
Alpine.store('auth', authStore);
Alpine.store('schedules', schedulesStore);
Alpine.store('tracker', trackerStore);

// Register components
Alpine.data('timeline', timeline);
Alpine.data('scheduleModal', scheduleModal);

// Initialize auth (load key from localStorage)
Alpine.store('auth').init();
Alpine.store('tracker').init();

// Start Alpine
Alpine.start();
