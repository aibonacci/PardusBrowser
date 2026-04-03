use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// JavaScript injected into every browser window page to add a navigation toolbar.
pub const BROWSER_TOOLBAR_JS: &str = r#"
(function() {
    if (window.__pardusToolbar) return;
    window.__pardusToolbar = true;

    var INSTANCE_ID = '__INSTANCE_ID__';

    function emit(name, data) {
        try { window.__TAURI__.event.emit(name, data); } catch(e) {
            try { window.__TAURI_INTERNALS__.postMessage({ cmd: 'event', event: name, payload: JSON.stringify(data) }); } catch(e2) {}
        }
    }

    function createToolbar() {
        var bar = document.createElement('div');
        bar.id = 'pardus-toolbar';
        bar.style.cssText = 'position:fixed;top:0;left:0;right:0;height:40px;z-index:2147483647;'
            + 'background:#161b22;border-bottom:1px solid #30363d;display:flex;align-items:center;'
            + 'padding:0 8px;gap:6px;font-family:-apple-system,BlinkMacSystemFont,sans-serif;';

        var btnStyle = 'background:#30363d;border:none;color:#e6edf3;border-radius:4px;padding:4px 10px;'
            + 'font-size:12px;cursor:pointer;height:28px;line-height:20px;';

        // Back button
        var back = document.createElement('button');
        back.textContent = '\u2190';
        back.style.cssText = btnStyle;
        back.onclick = function() { window.history.back(); };
        bar.appendChild(back);

        // Forward button
        var fwd = document.createElement('button');
        fwd.textContent = '\u2192';
        fwd.style.cssText = btnStyle;
        fwd.onclick = function() { window.history.forward(); };
        bar.appendChild(fwd);

        // Refresh button
        var ref = document.createElement('button');
        ref.textContent = '\u21BB';
        ref.style.cssText = btnStyle;
        ref.onclick = function() { window.location.reload(); };
        bar.appendChild(ref);

        // URL input
        var input = document.createElement('input');
        input.type = 'text';
        input.value = window.location.href;
        input.style.cssText = 'flex:1;height:28px;background:#0d1117;border:1px solid #30363d;'
            + 'border-radius:4px;color:#e6edf3;padding:0 8px;font-size:12px;'
            + 'font-family:\'SF Mono\',\'Cascadia Code\',monospace;outline:none;';
        input.onfocus = function() { input.select(); };
        input.onkeydown = function(e) {
            if (e.key === 'Enter') {
                var url = input.value.trim();
                if (url && !url.match(/^https?:\/\//)) url = 'https://' + url;
                emit('browser-navigate', { instance_id: INSTANCE_ID, url: url });
            }
        };
        bar.appendChild(input);

        // Update input on URL change
        var origPush = history.pushState;
        history.pushState = function() {
            origPush.apply(this, arguments);
            input.value = window.location.href;
            emit('browser-url-changed', { instance_id: INSTANCE_ID, url: window.location.href });
        };
        var origReplace = history.replaceState;
        history.replaceState = function() {
            origReplace.apply(this, arguments);
            input.value = window.location.href;
            emit('browser-url-changed', { instance_id: INSTANCE_ID, url: window.location.href });
        };
        window.addEventListener('popstate', function() {
            input.value = window.location.href;
            emit('browser-url-changed', { instance_id: INSTANCE_ID, url: window.location.href });
        });

        document.documentElement.appendChild(bar);
        document.body.style.paddingTop = '40px';

        emit('browser-url-changed', { instance_id: INSTANCE_ID, url: window.location.href });
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', createToolbar);
    } else {
        createToolbar();
    }
})();
"#;

/// JavaScript injected when a CAPTCHA challenge is detected — adds an urgent banner.
pub const CHALLENGE_BANNER_JS: &str = r#"
(function() {
    if (window.__pardusChallengeBanner) return;
    window.__pardusChallengeBanner = true;

    var banner = document.createElement('div');
    banner.id = 'pardus-challenge-banner';
    banner.style.cssText = 'position:fixed;top:40px;left:0;right:0;z-index:2147483646;'
        + 'background:linear-gradient(135deg,#ff6b35,#f7931e);color:#fff;'
        + 'padding:10px 20px;font-family:system-ui,sans-serif;font-size:14px;font-weight:600;'
        + 'display:flex;align-items:center;justify-content:space-between;'
        + 'box-shadow:0 4px 12px rgba(0,0,0,0.3);';
    banner.innerHTML = '<span>\u26A0\uFE0F CAPTCHA DETECTED \u2014 Solve the challenge to let the agent continue</span>'
        + '<button onclick="this.parentElement.remove();document.body.style.paddingTop=\'40px\';" '
        + 'style="background:rgba(255,255,255,0.2);border:none;color:#fff;padding:4px 12px;'
        + 'border-radius:4px;cursor:pointer;font-size:12px;">Dismiss</button>';
    document.documentElement.appendChild(banner);
    document.body.style.paddingTop = '80px';
})();
"#;

/// Open a browser window for the given instance.
pub fn open_browser_window(
    app_handle: &AppHandle,
    instance_id: &str,
    url: &str,
) -> Result<String, String> {
    let label = format!("browser-{}", instance_id);

    // Close existing window if any
    if let Some(existing) = app_handle.get_webview_window(&label) {
        let _ = existing.close();
    }

    let parsed_url: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;

    let toolbar_js = BROWSER_TOOLBAR_JS.replace("__INSTANCE_ID__", instance_id);

    let _window = WebviewWindowBuilder::new(
        app_handle,
        &label,
        WebviewUrl::External(parsed_url),
    )
    .title("Pardus Browser")
    .inner_size(1200.0, 800.0)
    .resizable(true)
    .initialization_script(&toolbar_js)
    .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15")
    .build()
    .map_err(|e| e.to_string())?;

    Ok(label)
}

/// Close a browser window for the given instance.
pub fn close_browser_window(
    app_handle: &AppHandle,
    instance_id: &str,
) -> Result<(), String> {
    let label = format!("browser-{}", instance_id);
    if let Some(window) = app_handle.get_webview_window(&label) {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Inject the challenge banner into a browser window.
pub fn inject_challenge_banner(
    app_handle: &AppHandle,
    instance_id: &str,
) -> Result<(), String> {
    let label = format!("browser-{}", instance_id);
    if let Some(window) = app_handle.get_webview_window(&label) {
        window.eval(CHALLENGE_BANNER_JS).map_err(|e| e.to_string())?;
    }
    Ok(())
}
