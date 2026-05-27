const API = window.location.origin;
let messages = [];

// ── 导航 ──
document.querySelectorAll('.nav-item').forEach(el => {
  el.addEventListener('click', () => {
    document.querySelectorAll('.nav-item').forEach(e => e.classList.remove('active'));
    document.querySelectorAll('.panel').forEach(e => e.classList.remove('active'));
    el.classList.add('active');
    document.getElementById('panel-' + el.dataset.panel).classList.add('active');
  });
});

// ── 对话 ──
const input = document.getElementById('chat-input');
input.addEventListener('keydown', async (e) => {
  if (e.key === 'Enter' && input.value.trim()) {
    const text = input.value.trim();
    input.value = '';
    addMsg('user', text);
    addMsg('system', '⏳ 思考中...');
    messages.push({role:'user',content:text});
    try {
      const resp = await fetch(API+'/api/chat', {
        method:'POST', headers:{'Content-Type':'application/json'},
        body: JSON.stringify({ model: localStorage.getItem('bugs_model') || '', messages })
      });
      const data = await resp.json();
      document.querySelector('.msg:last-child').remove();
      if (data.content) { addMsg('assistant', data.content); messages.push({role:'assistant',content:data.content}); }
      else if (data.error) addMsg('system', '❌ '+data.error);
    } catch(e) { document.querySelector('.msg:last-child').remove(); addMsg('system','❌ 连接失败'); }
  }
});

function addMsg(role, text) {
  const div = document.createElement('div');
  div.className = 'msg ' + role;
  const now = new Date();
  div.innerHTML = `<span class="msg-time">${now.getHours().toString().padStart(2,'0')}:${now.getMinutes().toString().padStart(2,'0')}</span> ${text}`;
  document.getElementById('messages').appendChild(div);
  div.scrollIntoView({behavior:'smooth'});
}

// ── 场景 ──
async function fetchScenes() {
  try {
    const resp = await fetch(API+'/api/scenes');
    const data = await resp.json();
    const list = document.getElementById('scene-list');
    list.innerHTML = '';
    (data.scenes||[]).forEach(s => {
      const div = document.createElement('div');
      div.className = 'scene-item' + (s.name === data.current ? ' current' : '');
      div.textContent = '📁 ' + (s.name||'?');
      list.appendChild(div);
    });
  } catch(e) {}
}

// ── 信任 ──
async function fetchTrust() {
  try {
    const resp = await fetch(API+'/api/meditate/pending');
    const data = await resp.json();
    const list = document.getElementById('trust-list');
    list.innerHTML = '';
    (data.pending||[]).forEach(m => {
      const div = document.createElement('div');
      div.className = 'trust-item';
      div.textContent = `⚠️ ${m.category||'?'}  score:${m.strength||0}`;
      list.appendChild(div);
    });
    if (!data.pending||!data.pending.length) list.innerHTML = '<div class="trust-item">✅ 无待确认记忆</div>';
  } catch(e) {}
}

// ── 状态 ──
async function fetchStatus() {
  try {
    const resp = await fetch(API+'/api/status');
    document.getElementById('status-text').textContent = JSON.stringify(await resp.json(), null, 2);
  } catch(e) { document.getElementById('status-text').textContent = '连接失败'; }
}

// ── 初始化 ──
fetchScenes(); fetchTrust(); fetchStatus();
setInterval(() => { fetchScenes(); fetchTrust(); }, 15000);
