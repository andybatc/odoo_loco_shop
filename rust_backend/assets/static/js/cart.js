document.addEventListener('DOMContentLoaded', () => {
    const { createApp } = Vue;

    const userData = (() => {
        const el = document.getElementById('cart-user-data');
        if (!el) return {};
        try { return JSON.parse(el.textContent) || {}; } catch { return {}; }
    })();

    createApp({
        delimiters: ['[[', ']]'],
        data() {
            const rawTotal = document.getElementById('cart-app')?.getAttribute('data-total') || '0';
            let items = [];
            const scriptEl = document.getElementById('cart-data');
            if (scriptEl) {
                try {
                    items = JSON.parse(scriptEl.textContent);
                } catch (e) {
                    console.error('Error parsing cart data:', e);
                    items = [];
                }
                if (!Array.isArray(items)) items = [];
            }
            return {
                items,
                totalBackend: parseFloat(rawTotal),
                customer: {
                    country: userData.country || '',
                    state: userData.state || '',
                },
                shippingCost: null,
                shippingOrigin: '',
            }
        },
        computed: {
            totalGeneral() {
                return this.items.reduce((acc, item) => {
                    return acc + (item.price * item.quantity);
                }, 0);
            },
            totalTax() {
                let tax = 0;
                for (const item of this.items) {
                    if (item.tax_percent) {
                        const lineTotal = item.price * item.quantity;
                        const rate = parseFloat(item.tax_percent);
                        tax += lineTotal - (lineTotal / (1 + rate / 100));
                    }
                }
                return tax;
            },
            taxLabel() {
                const rates = new Set();
                for (const item of this.items) {
                    if (item.tax_percent) rates.add(item.tax_percent);
                }
                if (rates.size === 1) {
                    const pct = parseFloat([...rates][0]);
                    return `(${pct}%)`;
                }
                return '';
            },
            totalWithShipping() {
                if (this.noShipping) return this.totalGeneral.toFixed(2);
                const base = this.totalGeneral;
                const ship = this.shippingCost !== null ? parseFloat(this.shippingCost) : 0;
                return (base + ship).toFixed(2);
            },
            noShipping() {
                return this.shippingCost === 0 && (!this.shippingOrigin || this.shippingOrigin.includes('Sin origen'));
            },
        },
        methods: {
            handleImageError(event) {
                event.target.src = '/static/images/No image available.jpeg';
            },
            async updateQuantity(item, newQty) {
                if (newQty < 1) return;
                try {
                    const res = await fetch(`/api/carts/items/${item.item_id}`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ quantity: newQty })
                    });
                    if (!res.ok) throw new Error('Error al actualizar');
                    item.quantity = newQty;
                    window.dispatchEvent(new CustomEvent('update-cart-count'));
                    this.shippingCost = null;
                    this.shippingOrigin = '';
                    this.estimateShipping();
                } catch(e) {
                    console.error('Error updating quantity:', e);
                    showToast('Error al actualizar cantidad');
                }
            },
            async removeItem(item) {
                if (!confirm('¿Eliminar este producto del carrito?')) return;
                try {
                    const res = await fetch(`/api/carts/items/${item.item_id}`, {
                        method: 'DELETE'
                    });
                    if (!res.ok) throw new Error('Error al eliminar');
                    const idx = this.items.indexOf(item);
                    if (idx > -1) this.items.splice(idx, 1);
                    window.dispatchEvent(new CustomEvent('update-cart-count'));
                    this.shippingCost = null;
                    this.shippingOrigin = '';
                    this.estimateShipping();
                } catch(e) {
                    console.error('Error removing item:', e);
                    showToast('Error al eliminar producto');
                }
            },
            async estimateShipping() {
                const country = this.customer.country;
                const state = this.customer.state;
                const ids = this.items.map(i => i.id).join(',');
                if (!country || !state || !ids) {
                    this.shippingCost = null;
                    this.shippingOrigin = '';
                    return;
                }
                try {
                    const resp = await fetch(`/api/shipping/estimate?product_ids=${ids}&country=${encodeURIComponent(country)}&state=${encodeURIComponent(state)}`);
                    const data = await resp.json();
                    this.shippingCost = parseFloat(data.shipping_cost) || 0;
                    this.shippingOrigin = data.origin_summary || '';
                } catch {
                    this.shippingCost = null;
                    this.shippingOrigin = '';
                }
            },
        },
        mounted() {
            if (this.customer.country && this.customer.state && this.items.length > 0) {
                this.estimateShipping();
            }
        }
    }).mount('#cart-app');
});