const { invoke } = window.__TAURI__.core;
const { getCurrentWebviewWindow } = window.__TAURI__.webviewWindow;

window.onerror = function (msg, url, line, col, error) {
  invoke('js_log', { message: `JS ERROR: ${msg} at ${line}:${col}` });
};

const appWindow = getCurrentWebviewWindow();
const settingsUI = document.getElementById('settings-ui');
const overlayContainer = document.getElementById('overlay-container');
const hole = document.getElementById('hole');
const blurRange = document.getElementById('blur-range');
const blurValue = document.getElementById('blur-value');
const closeBtn = document.getElementById('close-btn');

const maskTop = document.getElementById('mask-top');
const maskBottom = document.getElementById('mask-bottom');
const maskLeft = document.getElementById('mask-left');
const maskRight = document.getElementById('mask-right');

let scaleFactor = 1.0;

let isOverlayMode = false;

async function updateHole() {
  if (appWindow.label !== 'overlay') return;

  try {
    // スケール係数はモニタ移動で変わるため、常に最新を取得
    scaleFactor = await appWindow.scaleFactor();

    const info = await invoke('get_active_window');
    if (info && info.rect) {
      const rect = info.rect;

      const left = Math.round(rect.left / scaleFactor);
      const top = Math.round(rect.top / scaleFactor);
      const width = Math.round((rect.right - rect.left) / scaleFactor);
      const height = Math.round((rect.bottom - rect.top) / scaleFactor);
      const right = left + width;
      const bottom = top + height;

      // 4つのマスクで穴を作る
      maskTop.style.display = 'block';
      maskTop.style.top = '0';
      maskTop.style.left = '0';
      maskTop.style.width = '100vw';
      maskTop.style.height = top + 'px';

      maskBottom.style.display = 'block';
      maskBottom.style.top = bottom + 'px';
      maskBottom.style.left = '0';
      maskBottom.style.width = '100vw';
      maskBottom.style.height = `calc(100vh - ${bottom}px)`;

      maskLeft.style.display = 'block';
      maskLeft.style.top = top + 'px';
      maskLeft.style.left = '0';
      maskLeft.style.width = left + 'px';
      maskLeft.style.height = height + 'px';

      maskRight.style.display = 'block';
      maskRight.style.top = top + 'px';
      maskRight.style.left = right + 'px';
      maskRight.style.width = `calc(100vw - ${right}px)`;
      maskRight.style.height = height + 'px';

      // デバッグ用の穴の枠線
      hole.style.display = 'block';
      hole.style.left = left + 'px';
      hole.style.top = top + 'px';
      hole.style.width = width + 'px';
      hole.style.height = height + 'px';

      invoke('js_log', { message: `Mask update: ${left},${top} ${width}x${height}` });
    } else {
      [maskTop, maskBottom, maskLeft, maskRight, hole].forEach(el => el.style.display = 'none');
    }
  } catch (e) {
    invoke('js_log', { message: `Error in updateHole: ${e}` });
  }
}

// 初期設定
async function init() {
  invoke('js_log', { message: `Init called` });
  try {
    scaleFactor = await appWindow.scaleFactor();
    invoke('js_log', { message: `Init - window label: ${appWindow.label}, scale: ${scaleFactor}` });

    if (appWindow.label === 'overlay') {
      settingsUI.classList.add('hidden');
      overlayContainer.classList.remove('hidden');
      setInterval(updateHole, 100);
    } else {
      settingsUI.classList.remove('hidden');
      overlayContainer.classList.add('hidden');
    }
  } catch (e) {
    invoke('js_log', { message: `Init failed: ${e}` });
  }
}

blurRange.addEventListener('input', (e) => {
  blurValue.textContent = e.target.value + 'px';
  // 将来的にシェーダパラメータを更新する
});

closeBtn.addEventListener('click', () => {
  appWindow.hide();
});

init();
