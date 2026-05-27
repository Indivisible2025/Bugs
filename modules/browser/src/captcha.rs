//! 免费验证码处理 — 拟人化操作
//! 所有操作模拟真实人类：鼠标轨迹、延迟、微小误差

/// 检测页面是否含验证码
pub const CAPTCHA_DETECT_JS: &str = r##"
(function(){
    var result = { detected: false, type: '' };
    if (document.querySelector('.h-captcha, [data-hcaptcha]')) result = { detected: true, type: 'hcaptcha' };
    else if (document.querySelector('.g-recaptcha, [data-sitekey]')) result = { detected: true, type: 'recaptcha' };
    else if (document.querySelector('[data-turnstile]')) result = { detected: true, type: 'turnstile' };
    return JSON.stringify(result);
})()
"##;

/// 拟人化点击验证码 — 贝塞尔鼠标轨迹 + 随机延迟 + 位置微偏移
pub const CAPTCHA_HUMAN_CLICK_JS: &str = r##"
(function(){
    function findCheckbox() {
        // reCAPTCHA iframe 内的 checkbox
        var frames = document.querySelectorAll('iframe');
        for (var i = 0; i < frames.length; i++) {
            try {
                if (!frames[i].src || !frames[i].src.includes('recaptcha')) continue;
                var doc = frames[i].contentDocument || frames[i].contentWindow.document;
                var cb = doc.querySelector('.recaptcha-checkbox-border, #recaptcha-anchor');
                if (!cb) continue;
                var rect = cb.getBoundingClientRect();
                var frameRect = frames[i].getBoundingClientRect();
                return {
                    x: frameRect.x + rect.x + rect.width/2,
                    y: frameRect.y + rect.y + rect.height/2,
                    element: cb
                };
            } catch(e) {}
        }
        return null;
    }

    function humanClick(target, callback) {
        if (!target) return false;
        var destX = target.x + (Math.random() - 0.5) * 6;  // ±3px 位置微偏移
        var destY = target.y + (Math.random() - 0.5) * 6;
        var startX = Math.random() * window.innerWidth;
        var startY = Math.random() * window.innerHeight;
        var steps = 12 + Math.floor(Math.random() * 16); // 12-28步

        var step = 0;
        function moveNext() {
            if (step > steps) {
                // 到达后随机延迟 50-300ms 再点击（人类反应时间）
                setTimeout(function() {
                    target.element.dispatchEvent(new MouseEvent('mousedown', {
                        clientX: destX, clientY: destY, bubbles: true
                    }));
                    // mousedown 和 mouseup 间隔 40-120ms（人类按压时间）
                    setTimeout(function() {
                        target.element.dispatchEvent(new MouseEvent('mouseup', {
                            clientX: destX, clientY: destY, bubbles: true
                        }));
                        target.element.dispatchEvent(new MouseEvent('click', {
                            clientX: destX, clientY: destY, bubbles: true
                        }));
                        if (callback) callback(true);
                    }, 40 + Math.random() * 80);
                }, 50 + Math.random() * 250);
                return;
            }
            var t = step / steps;
            var easeT = t < 0.5 ? 4*t*t*t : 1 - Math.pow(-2*t + 2, 3)/2; // easeInOutCubic
            var currX = startX + (destX - startX) * easeT + (Math.random() - 0.5) * 4;
            var currY = startY + (destY - startY) * easeT + (Math.random() - 0.5) * 4;
            document.dispatchEvent(new MouseEvent('mousemove', {
                clientX: currX, clientY: currY, bubbles: true
            }));
            step++;
            // 每步间隔 15-40ms（自然移动速度）
            setTimeout(moveNext, 15 + Math.random() * 25);
        }
        moveNext();
        return true;
    }

    var target = findCheckbox();
    if (target) {
        // 有时先在周围"犹豫"一下（30%概率hover附近）
        if (Math.random() < 0.3) {
            setTimeout(function() {
                var hoverX = target.x + (Math.random() - 0.5) * 50;
                var hoverY = target.y + (Math.random() - 0.5) * 20;
                document.dispatchEvent(new MouseEvent('mousemove', {
                    clientX: hoverX, clientY: hoverY, bubbles: true
                }));
                setTimeout(function() {
                    humanClick(target, function(success) {
                        console.log('captcha human click:', success);
                    });
                }, 200 + Math.random() * 400);
            }, 100 + Math.random() * 300);
        } else {
            humanClick(target, function(success) {
                console.log('captcha human click:', success);
            });
        }
    }
    return target ? 'clicking' : 'no target';
})()
"##;
