// Auth store - manages API key authentication
export default {
    storageKey: 'satomat_api_key',
    key: '',
    modalOpen: false,
    modalInputValue: '',

    init() {
        this.key = localStorage.getItem(this.storageKey) || '';
    },

    // Method instead of getter for Alpine compatibility
    hasKey() {
        return !!this.key;
    },

    showModal() {
        this.modalInputValue = this.key;
        this.modalOpen = true;
    },

    hideModal() {
        this.modalOpen = false;
    },

    save() {
        const trimmed = this.modalInputValue.trim();
        if (trimmed) {
            this.key = trimmed;
            localStorage.setItem(this.storageKey, trimmed);
            this.hideModal();
        }
    },

    clear() {
        this.key = '';
        this.modalInputValue = '';
        localStorage.removeItem(this.storageKey);
        this.hideModal();
    },

    async fetch(url, options = {}) {
        if (!this.key) {
            throw new Error('No API key configured');
        }
        return fetch(url, {
            ...options,
            headers: {
                ...options.headers,
                'Authorization': `Bearer ${this.key}`,
            },
        });
    },
};
