//! 行为模拟 — 鼠标轨迹、键盘时序、滚动模式、人类反应延迟

/// 人类行为模拟引擎
/// 在页面加载后注入，模拟真实用户操作

/// 注入行为模拟——让页面"觉得"真人在用
pub const BEHAVIOR_JS: &str = r##"
(function(){
    // ===== 1. 鼠标轨迹模拟（贝塞尔曲线+噪声） =====
    let mouseX = Math.random() * window.innerWidth;
    let mouseY = Math.random() * window.innerHeight;

    function simulateMouseMove(targetX, targetY) {
        const steps = 15 + Math.floor(Math.random() * 20);
        const dx = targetX - mouseX;
        const dy = targetY - mouseY;
        for (let i = 0; i <= steps; i++) {
            const t = i / steps;
            const easeT = t < 0.5 ? 4*t*t*t : 1 - Math.pow(-2*t + 2, 3)/2; // easeInOutCubic
            const currentX = mouseX + dx * easeT + (Math.random() - 0.5) * 8;
            const currentY = mouseY + dy * easeT + (Math.random() - 0.5) * 8;
            document.dispatchEvent(new MouseEvent('mousemove', {
                clientX: currentX, clientY: currentY,
                screenX: currentX, screenY: currentY + 40,
                bubbles: true, cancelable: true
            }));
        }
        mouseX = targetX;
        mouseY = targetY;
    }

    // ===== 2. 自然滚动模式（分段+暂停） =====
    let scrollInterval = null;
    function startNaturalScroll() {
        let scrollAmount = 0;
        const totalScroll = 200 + Math.random() * window.innerHeight * 0.6;
        scrollInterval = setInterval(() => {
            if (scrollAmount >= totalScroll) {
                clearInterval(scrollInterval);
                // 偶尔回滚一点（人类行为）
                if (Math.random() > 0.7) {
                    window.scrollBy(0, -(30 + Math.random() * 80));
                }
                return;
            }
            const step = 10 + Math.random() * 20;
            scrollAmount += step;
            window.scrollBy(0, step);
            document.dispatchEvent(new Event('scroll', { bubbles: true }));
            // 模拟阅读暂停
            if (Math.random() > 0.85) {
                clearInterval(scrollInterval);
                setTimeout(() => startNaturalScroll(), 500 + Math.random() * 2000);
            }
        }, 80 + Math.random() * 120);
    }

    // ===== 3. 键盘时序（非匀速+偶尔停顿） =====
    function simulateTyping(element, text, callback) {
        let index = 0;
        function typeNext() {
            if (index >= text.length) { if (callback) callback(); return; }
            const char = text[index];
            const baseDelay = 40 + Math.random() * 80; // 40-120ms per char (realistic typing)
            // 空格和特殊字符更慢
            const delay = (char === ' ' || ',.?!'.includes(char)) ? baseDelay * 2.5 : baseDelay;
            // 偶尔"想一下"（3%概率停顿200-800ms）
            const pause = Math.random() < 0.03;
            element.value += char;
            element.dispatchEvent(new Event('input', { bubbles: true }));
            element.dispatchEvent(new KeyboardEvent('keydown', { key: char, bubbles: true }));
            element.dispatchEvent(new KeyboardEvent('keypress', { key: char, bubbles: true }));
            element.dispatchEvent(new KeyboardEvent('keyup', { key: char, bubbles: true }));
            index++;
            setTimeout(typeNext, pause ? delay + 200 + Math.random() * 600 : delay);
        }
        typeNext();
    }

    // ===== 4. 鼠标移动进入页面 + 初始移动 =====
    const centerX = window.innerWidth / 2;
    const centerY = window.innerHeight / 2;
    setTimeout(() => simulateMouseMove(centerX - 100 + Math.random() * 200, 80 + Math.random() * 100), 300);
    setTimeout(() => simulateMouseMove(centerX + Math.random() * 400, centerY + Math.random() * 300), 1500);

    // ===== 5. 暴露行为 API 到 window =====
    window.__bugs_behavior = {
        simulateMouseMove: simulateMouseMove,
        startNaturalScroll: startNaturalScroll,
        simulateTyping: simulateTyping,
    };

    // ===== 6. 页面加载后自动开始自然滚动 =====
    setTimeout(startNaturalScroll, 2000 + Math.random() * 3000);

    // ===== 7. 定期随机鼠标微动（防僵死检测） =====
    setInterval(() => {
        if (Math.random() > 0.6) return;
        const jitterX = mouseX + (Math.random() - 0.5) * 30;
        const jitterY = mouseY + (Math.random() - 0.5) * 30;
        document.dispatchEvent(new MouseEvent('mousemove', {
            clientX: jitterX, clientY: jitterY,
            screenX: jitterX, screenY: jitterY + 40,
            bubbles: true, cancelable: true
        }));
    }, 1500 + Math.random() * 3000);

    // ===== 8. 模拟随机 focus/blur 事件 =====
    window.addEventListener('load', () => {
        setTimeout(() => { window.dispatchEvent(new Event('focus')); }, 1000);
    });
})()
"##;

/// 搜索页行为——输入关键词+按回车
pub const SEARCH_BEHAVIOR_JS: &str = r##"
(function(){
    // 找到搜索框
    const inputs = document.querySelectorAll('input[type="text"], input[type="search"], textarea, [role="combobox"]');
    const searchBox = inputs.length > 0 ? inputs[0] : null;
    if (!searchBox) return;

    // 模拟点击搜索框
    const rect = searchBox.getBoundingClientRect();
    const clickX = rect.x + rect.width * (0.3 + Math.random() * 0.4);
    const clickY = rect.y + rect.height / 2;
    searchBox.dispatchEvent(new MouseEvent('click', { clientX: clickX, clientY: clickY, bubbles: true }));
    searchBox.focus();

    // 模拟打字（如果有 __bugs_behavior）
    if (window.__bugs_behavior && window.__bugs_behavior.simulateTyping) {
        const query = new URLSearchParams(window.location.search).get('q') || '';
        if (query) {
            window.__bugs_behavior.simulateTyping(searchBox, query, () => {
                // 打完后模拟回车
                const enterEvent = new KeyboardEvent('keydown', { key: 'Enter', keyCode: 13, bubbles: true });
                searchBox.dispatchEvent(enterEvent);
            });
        }
    }
})()
"##;
