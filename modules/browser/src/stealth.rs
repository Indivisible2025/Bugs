//! 反检测脚本 — 借鉴 puppeteer-extra-plugin-stealth
//! 在新页面加载前注入，修改浏览器暴露的所有可检测属性

/// 综合反检测脚本——在新页面 URL 加载前立即执行
pub const STEALTH_JS: &str = r##"
(function(){
    // ===== 1. navigator.webdriver — 最致命的破绽 =====
    Object.defineProperty(navigator, 'webdriver', { get: () => false });

    // ===== 2. navigator.plugins — headless 默认为空数组 =====
    Object.defineProperty(navigator, 'plugins', {
        get: () => {
            const arr = [1, 2, 3, 4, 5];
            arr.item = () => null;
            arr.namedItem = () => null;
            arr.refresh = () => {};
            return arr;
        }
    });

    // ===== 3. navigator.languages — 补充语言列表 =====
    Object.defineProperty(navigator, 'languages', {
        get: () => ['zh-CN', 'zh', 'en-US', 'en']
    });

    // ===== 4. navigator.hardwareConcurrency — 真实CPU核心数 =====
    Object.defineProperty(navigator, 'hardwareConcurrency', {
        get: () => Math.max(4, Math.floor(Math.random() * 8) + 4)
    });

    // ===== 5. navigator.deviceMemory =====
    Object.defineProperty(navigator, 'deviceMemory', {
        get: () => Math.random() > 0.5 ? 8 : 16
    });

    // ===== 6. Canvas 指纹噪声 — 每次渲染加微小偏移 =====
    const origToDataURL = HTMLCanvasElement.prototype.toDataURL;
    HTMLCanvasElement.prototype.toDataURL = function(type) {
        const ctx = this.getContext('2d');
        if (ctx) {
            const noise = 0.1 + Math.random() * 0.9;
            const imageData = ctx.getImageData(0, 0, this.width, this.height);
            for (let i = 0; i < imageData.data.length; i += 4) {
                imageData.data[i] = Math.min(255, Math.max(0, imageData.data[i] + (Math.random() - 0.5) * noise));
            }
            ctx.putImageData(imageData, 0, 0);
        }
        return origToDataURL.apply(this, arguments);
    };

    // ===== 7. WebGL 指纹 — 伪装为常见GPU =====
    const origGetParameter = WebGLRenderingContext.prototype.getParameter;
    WebGLRenderingContext.prototype.getParameter = function(p) {
        if (p === 37445) return 'Intel Inc.';      // UNMASKED_VENDOR_WEBGL
        if (p === 37446) return 'Intel Iris OpenGL Engine'; // UNMASKED_RENDERER_WEBGL
        return origGetParameter.call(this, p);
    };
    if (typeof WebGL2RenderingContext !== 'undefined') {
        WebGL2RenderingContext.prototype.getParameter = WebGLRenderingContext.prototype.getParameter;
    }

    // ===== 8. window.chrome — 真实Chrome才有 =====
    if (!window.chrome) {
        window.chrome = {
            runtime: {},
            loadTimes: () => {},
            csi: () => {},
            app: {}
        };
    }

    // ===== 9. navigator.permissions — headless 行为异常 =====
    const origQuery = window.navigator.permissions.query;
    window.navigator.permissions.query = function(parameters) {
        if (parameters.name === 'notifications') {
            return Promise.resolve({ state: Notification.permission, onchange: null });
        }
        return origQuery.call(this, parameters);
    };

    // ===== 10. AudioContext 指纹 — 添加微小噪声 =====
    if (typeof AudioContext !== 'undefined') {
        const origCreateOscillator = AudioContext.prototype.createOscillator;
        AudioContext.prototype.createOscillator = function() {
            const osc = origCreateOscillator.call(this);
            const origStart = osc.start.bind(osc);
            osc.start = function() { return origStart(); };
            return osc;
        };
    }

    // ===== 11. screen 属性 — 匹配窗口尺寸 =====
    Object.defineProperty(screen, 'colorDepth', { get: () => 24 });
    Object.defineProperty(screen, 'pixelDepth', { get: () => 24 });

    // ===== 12. Connection 类型 =====
    if (navigator.connection) {
        Object.defineProperty(navigator.connection, 'rtt', { get: () => Math.random() * 50 + 50 });
    }

    // ===== 13. 防止通过 iframe 检测 =====
    if (window.top === window) {
        Object.defineProperty(document, 'hidden', { get: () => false });
        Object.defineProperty(document, 'visibilityState', { get: () => 'visible' });
    }
})()
"##;
