fn file_transfer_theme_variants(theme: &str) -> (&'static str, &'static str, &'static str) {
    match theme {
        "sticky-note" => (
            r#"
            --bg-body: #fff4b8;
            --bg-panel: rgba(255, 249, 211, 0.9);
            --bg-input: rgba(255, 255, 255, 0.74);
            --bg-button: rgba(255, 255, 255, 0.3);
            --border-dark: #c7a35d;
            --text-primary: #5d4037;
            --text-secondary: #7a5c50;
            --accent-color: #f0a70b;
            --shadow-color: rgba(93, 64, 55, 0.16);
            --font-mono: "Consolas", "Courier New", monospace;
            --content-font-family: "Comic Sans MS", "Chalkboard SE", "Segoe Print", "Microsoft YaHei", sans-serif;
            --radius: 12px;
            --bubble-received-bg: #fffde7;
            --panel-border: 1px solid rgba(199, 163, 93, 0.28);
            --panel-radius: 14px;
            --panel-shadow: 0 10px 22px rgba(93, 64, 55, 0.08);
            --input-border: 1px dashed rgba(240, 167, 11, 0.7);
            --input-radius: 8px;
            --input-shadow: none;
            --button-border: none;
            --button-radius: 999px;
            --button-shadow: none;
            --button-active-transform: scale(0.98);
            --button-active-shadow: none;
            --button-active-filled-shadow: 0 10px 22px rgba(240, 167, 11, 0.24);
            --send-button-background: rgba(240, 167, 11, 0.18);
            --send-button-color: var(--text-primary);
            --send-button-border: 1px dashed rgba(240, 167, 11, 0.44);
            --send-button-shadow: 0 10px 22px rgba(240, 167, 11, 0.18);
            "#,
            r#"
            --bg-body: #242427;
            --bg-panel: rgba(47, 47, 51, 0.9);
            --bg-input: rgba(255, 255, 255, 0.08);
            --bg-button: rgba(255, 255, 255, 0.06);
            --border-dark: rgba(255, 212, 77, 0.24);
            --text-primary: #f2eee3;
            --text-secondary: #d0c7b5;
            --accent-color: #ffd44d;
            --shadow-color: rgba(0, 0, 0, 0.28);
            --bubble-received-bg: #3a3a3c;
            --panel-border: 1px solid rgba(255, 212, 77, 0.16);
            --panel-shadow: 0 14px 28px rgba(0, 0, 0, 0.22);
            --input-border: 1px dashed rgba(255, 212, 77, 0.42);
            --send-button-background: rgba(255, 212, 77, 0.14);
            --send-button-border: 1px dashed rgba(255, 212, 77, 0.3);
            --send-button-shadow: 0 10px 22px rgba(0, 0, 0, 0.2);
            "#,
            r#"
            body.theme-sticky-note {{
                background-color: var(--bg-body);
                background-image: linear-gradient(180deg, rgba(255, 255, 255, 0.4), rgba(255, 243, 188, 0.22));
                background-size: 100% 100%;
            }}
            body.theme-sticky-note.dark-mode {{
                background-image: linear-gradient(180deg, rgba(0, 0, 0, 0.4), rgba(0, 0, 0, 0.22));
            }}
            @media (prefers-color-scheme: dark) {{
                body.theme-sticky-note:not(.light-mode) {{
                    background-image: linear-gradient(180deg, rgba(0, 0, 0, 0.4), rgba(0, 0, 0, 0.22));
                }}
            }}
            .theme-sticky-note .avatar,
            .theme-sticky-note .bubble,
            .theme-sticky-note .text-input,
            .theme-sticky-note .retro-btn {
                border-style: solid !important;
            }
            .theme-sticky-note .message.received .bubble {
                transform: rotate(-0.5deg);
                box-shadow: 2px 2px 0 var(--shadow-color);
            }
            .theme-sticky-note .message.sent .bubble {
                transform: rotate(0.8deg);
                box-shadow: -2px 2px 0 var(--shadow-color);
            }
            "#,
        ),
        "paper" => (
            r#"
            --bg-body: #f4ecd8;
            --bg-window: #fdf6e3;
            --bg-panel: rgba(255, 253, 247, 0.78);
            --bg-input: #ffffff;
            --bg-button: rgba(139, 90, 43, 0.08);
            --border-dark: #d5c4a1;
            --text-primary: #3c3836;
            --text-secondary: #7c6f64;
            --accent-color: #8b5a2b;
            --shadow-color: rgba(60, 56, 54, 0.12);
            --font-mono: "Courier New", Courier, monospace;
            --content-font-family: "Georgia", "STSong", "SimSun", "Songti SC", serif;
            --radius: 2px;
            --bubble-received-bg: #fffdf7;
            --panel-border: 1px solid #d5c4a1;
            --panel-radius: 2px;
            --panel-shadow: 0 2px 10px rgba(0, 0, 0, 0.03);
            --input-border: 1px solid #d5c4a1;
            --input-radius: 2px;
            --input-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.03);
            --button-border: 1px solid rgba(139, 90, 43, 0.3);
            --button-radius: 4px;
            --send-button-background: rgba(139, 90, 43, 0.12);
            --send-button-color: var(--text-primary);
            --send-button-border: 1px solid rgba(139, 90, 43, 0.28);
            --send-button-shadow: 0 4px 10px rgba(139, 90, 43, 0.12);
            "#,
            r#"
            --bg-body: #282828;
            --bg-panel: rgba(50, 48, 47, 0.82);
            --bg-input: #1d2021;
            --bg-button: rgba(213, 196, 161, 0.1);
            --border-dark: #504945;
            --text-primary: #ebdbb2;
            --text-secondary: #a89984;
            --accent-color: #d79921;
            --shadow-color: rgba(0, 0, 0, 0.3);
            --bubble-received-bg: #32302f;
            --panel-border: 1px solid #504945;
            --input-border: 1px solid #504945;
            --send-button-background: rgba(215, 153, 33, 0.16);
            --send-button-color: var(--text-primary);
            --send-button-border: 1px solid rgba(215, 153, 33, 0.26);
            --send-button-shadow: 0 4px 10px rgba(0, 0, 0, 0.18);
            "#,
            r#"
            body.theme-paper {{
                background-image: linear-gradient(rgba(139, 90, 43, 0.06) 1px, transparent 1px);
                background-size: 100% 1.65em;
            }}
            body.theme-paper::before {{
                content: "";
                position: fixed;
                inset: 0;
                background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
                opacity: 0.04;
                pointer-events: none;
                z-index: 9999;
            }}
            @media (prefers-color-scheme: dark) {{
                body.theme-paper:not(.light-mode) {{
                    background-image: linear-gradient(rgba(213, 196, 161, 0.04) 1px, transparent 1px);
                }}
            }}
            "#,
        ),
        "mica" => (
            r#"
            --bg-body: #f3f3f3;
            --bg-panel: rgba(255, 255, 255, 0.48);
            --bg-input: rgba(255, 255, 255, 0.74);
            --bg-button: rgba(255, 255, 255, 0.46);
            --border-dark: rgba(128, 128, 128, 0.18);
            --text-primary: #1a2435;
            --text-secondary: #607188;
            --accent-color: #4f7dff;
            --shadow-color: rgba(15, 23, 42, 0.1);
            --font-mono: system-ui, -apple-system, sans-serif;
            --radius: 12px;
            --bubble-received-bg: rgba(255, 255, 255, 0.9);
            --panel-border: 1px solid rgba(255, 255, 255, 0.3);
            --send-button-background: #4f7dff;
            --send-button-color: #ffffff;
            "#,
            r#"
            --bg-body: #1a1a1a;
            --bg-panel: rgba(36, 36, 36, 0.52);
            --bg-input: rgba(255, 255, 255, 0.12);
            --bg-element: rgba(45, 45, 45, 0.65);
            --bg-button: rgba(255, 255, 255, 0.05);
            --text-primary: #e8e8e8;
            --text-secondary: #a8a8a8;
            --accent-color: #4f7dff;
            --border-dark: rgba(255, 255, 255, 0.08);
            --bubble-received-bg: rgba(40, 40, 40, 0.9);
            --send-button-background: #4f7dff;
            --send-button-color: #ffffff;
            "#,
            r#"
            .theme-mica header, .theme-mica footer {{
                backdrop-filter: blur(20px) saturate(180%);
                -webkit-backdrop-filter: blur(20px) saturate(180%);
            }}
            .theme-mica .message.received .bubble {{
                backdrop-filter: blur(10px);
                -webkit-backdrop-filter: blur(10px);
            }}
            .theme-mica .retro-btn.send-btn {{
                background: var(--send-button-background);
                color: var(--send-button-color);
                border: none;
                box-shadow: 0 4px 12px rgba(79, 125, 255, 0.3);
            }}
            "#,
        ),
        "acrylic" => (
            r#"
            --bg-body: #f3f3f3;
            --bg-panel: rgba(255, 255, 255, 0.34);
            --bg-input: rgba(255, 255, 255, 0.52);
            --bg-button: rgba(255, 255, 255, 0.2);
            --border-dark: rgba(255, 255, 255, 0.32);
            --text-primary: #162234;
            --text-secondary: #607188;
            --accent-color: #4f7dff;
            --shadow-color: rgba(15, 23, 42, 0.08);
            --font-mono: system-ui, -apple-system, sans-serif;
            --radius: 10px;
            --bubble-received-bg: rgba(255, 255, 255, 0.4);
            --send-button-background: #4f7dff;
            --send-button-color: #ffffff;
            "#,
            r#"
            --bg-body: #141414;
            --bg-panel: rgba(28, 28, 28, 0.38);
            --bg-input: rgba(255, 255, 255, 0.12);
            --bg-button: rgba(255, 255, 255, 0.06);
            --border-dark: rgba(255, 255, 255, 0.12);
            --text-primary: #e8e8e8;
            --text-secondary: #b2b2b2;
            --accent-color: #4f7dff;
            --bubble-received-bg: rgba(45, 45, 45, 0.42);
            --send-button-background: #4f7dff;
            --send-button-color: #ffffff;
            "#,
            r#"
            .theme-acrylic header, .theme-acrylic footer {{
                backdrop-filter: blur(30px) saturate(145%);
                -webkit-backdrop-filter: blur(30px) saturate(145%);
            }}
            .theme-acrylic .message.received .bubble {{
                backdrop-filter: blur(15px);
                -webkit-backdrop-filter: blur(15px);
            }}
            .theme-acrylic .retro-btn.send-btn {{
                background: var(--send-button-background);
                color: var(--send-button-color);
                border: none;
                box-shadow: 0 4px 12px rgba(79, 125, 255, 0.3);
            }}
            "#,
        ),
        "sakura" => (
            r#"
            --bg-body: #fff7fa;
            --bg-panel: rgba(255, 252, 253, 0.86);
            --bg-input: rgba(255, 255, 255, 0.9);
            --bg-button: rgba(255, 255, 255, 0.78);
            --border-dark: rgba(255, 133, 161, 0.28);
            --text-primary: #4a343a;
            --text-secondary: #946c78;
            --accent-color: #ff85a1;
            --shadow-color: rgba(92, 58, 68, 0.12);
            --font-mono: "Segoe UI", system-ui, -apple-system, "PingFang SC", sans-serif;
            --content-font-family: var(--font-mono);
            --radius: 14px;
            --bubble-received-bg: rgba(255, 255, 255, 0.9);
            --panel-border: 1px solid rgba(255, 133, 161, 0.18);
            --panel-radius: 16px;
            --panel-shadow: 0 12px 30px rgba(160, 80, 100, 0.08);
            --input-border: 1px solid rgba(255, 133, 161, 0.25);
            --input-radius: 12px;
            --input-shadow: inset 0 1px 2px rgba(255, 133, 161, 0.05);
            --button-border: 1px solid rgba(255, 133, 161, 0.2);
            --button-radius: 12px;
            --button-shadow: none;
            --send-button-background: #ff85a1;
            --send-button-color: #ffffff;
            --send-button-border: 1px solid rgba(255, 107, 142, 0.45);
            --send-button-shadow: 0 8px 20px rgba(255, 133, 161, 0.24);
            "#,
            r#"
            --bg-body: #241a1e;
            --bg-panel: rgba(48, 34, 40, 0.9);
            --bg-input: rgba(255, 255, 255, 0.08);
            --bg-button: rgba(255, 255, 255, 0.06);
            --border-dark: rgba(255, 255, 255, 0.08);
            --text-primary: #fcecef;
            --text-secondary: #d4a5b2;
            --accent-color: #ff9fb4;
            --shadow-color: rgba(0, 0, 0, 0.3);
            --bubble-received-bg: rgba(65, 45, 52, 0.88);
            --panel-border: 1px solid rgba(255, 255, 255, 0.06);
            --input-border: 1px solid rgba(255, 255, 255, 0.08);
            --button-border: 1px solid rgba(255, 255, 255, 0.06);
            --send-button-background: #e87088;
            "#,
            r#"
            body.theme-sakura {{
                background:
                    radial-gradient(circle at 12% 18%, rgba(255, 183, 197, 0.2) 0 3px, transparent 4px),
                    radial-gradient(circle at 86% 28%, rgba(248, 200, 220, 0.18) 0 4px, transparent 5px),
                    radial-gradient(circle at 72% 82%, rgba(255, 183, 197, 0.14) 0 3px, transparent 4px),
                    var(--bg-body);
            }}
            .theme-sakura header, .theme-sakura footer {{
                backdrop-filter: blur(18px) saturate(130%);
                -webkit-backdrop-filter: blur(18px) saturate(130%);
            }}
            "#,
        ),
        "terminal" => (
            r#"
            --bg-body: #0d1117;
            --bg-panel: #010409;
            --bg-input: #0d1117;
            --bg-button: transparent;
            --border-dark: #30363d;
            --text-primary: #e6edf3;
            --text-secondary: #8b949e;
            --accent-color: #3fb950;
            --shadow-color: rgba(0, 0, 0, 0.55);
            --font-mono: "SF Mono", Menlo, Monaco, "Cascadia Mono", Consolas, monospace;
            --content-font-family: var(--font-mono);
            --radius: 0;
            --bubble-received-bg: #161b22;
            --panel-border: 1px solid #30363d;
            --panel-radius: 0;
            --panel-shadow: none;
            --input-border: 1px solid #30363d;
            --input-radius: 0;
            --input-shadow: none;
            --button-border: 1px solid #30363d;
            --button-radius: 0;
            --button-shadow: none;
            --send-button-background: rgba(63, 185, 80, 0.14);
            --send-button-color: #56d364;
            --send-button-border: 1px solid rgba(63, 185, 80, 0.46);
            --send-button-shadow: none;
            "#,
            r#"
            --bg-body: #0d1117;
            --bg-panel: #010409;
            --bg-input: #0d1117;
            --bg-button: transparent;
            --border-dark: #30363d;
            --text-primary: #e6edf3;
            --text-secondary: #8b949e;
            --accent-color: #3fb950;
            --shadow-color: rgba(0, 0, 0, 0.55);
            --bubble-received-bg: #161b22;
            "#,
            r#"
            .theme-terminal header, .theme-terminal footer {{
                border-color: #21262d;
            }}
            .theme-terminal .bubble {{
                box-shadow: none;
            }}
            .theme-terminal .message.sent .bubble {{
                color: #56d364;
            }}
            "#,
        ),
        "receipt" => (
            r#"
            --bg-body: #faf8f3;
            --bg-panel: #fffef9;
            --bg-input: #ffffff;
            --bg-button: transparent;
            --border-dark: rgba(0, 0, 0, 0.2);
            --text-primary: #1e1e1e;
            --text-secondary: #6a6a6a;
            --accent-color: #2a2a2a;
            --shadow-color: rgba(0, 0, 0, 0.1);
            --font-mono: "Courier New", Courier, "SF Mono", monospace;
            --content-font-family: var(--font-mono);
            --radius: 0;
            --bubble-received-bg: #fffef9;
            --panel-border: 1px dashed rgba(0, 0, 0, 0.18);
            --panel-radius: 2px;
            --panel-shadow: none;
            --input-border: 1px dashed rgba(0, 0, 0, 0.2);
            --input-radius: 0;
            --input-shadow: none;
            --button-border: 1px dashed rgba(0, 0, 0, 0.25);
            --button-radius: 0;
            --button-shadow: none;
            --send-button-background: #2a2a2a;
            --send-button-color: #fffef9;
            --send-button-border: 1px dashed #2a2a2a;
            --send-button-shadow: none;
            "#,
            r#"
            --bg-body: #1c1c1c;
            --bg-panel: #242424;
            --bg-input: #1b1b1b;
            --bg-button: transparent;
            --border-dark: rgba(255, 255, 255, 0.18);
            --text-primary: #e8e8e8;
            --text-secondary: #a0a0a0;
            --accent-color: #e8e8e8;
            --shadow-color: rgba(0, 0, 0, 0.35);
            --bubble-received-bg: #242424;
            --panel-border: 1px dashed rgba(255, 255, 255, 0.16);
            --input-border: 1px dashed rgba(255, 255, 255, 0.18);
            --button-border: 1px dashed rgba(255, 255, 255, 0.22);
            --send-button-background: #e8e8e8;
            --send-button-color: #1c1c1c;
            --send-button-border: 1px dashed #e8e8e8;
            "#,
            r#"
            body.theme-receipt {{
                background-image: repeating-linear-gradient(
                    0deg,
                    transparent 0,
                    transparent 23px,
                    rgba(0, 0, 0, 0.025) 24px
                );
            }}
            .theme-receipt header, .theme-receipt footer {{
                border-style: dashed;
            }}
            .theme-receipt .bubble {{
                border-style: dashed;
                box-shadow: none;
            }}
            "#,
        ),
        "ink" => (
            r#"
            --bg-body: #f0ebe3;
            --bg-panel: rgba(247, 244, 238, 0.94);
            --bg-input: rgba(255, 255, 255, 0.72);
            --bg-button: transparent;
            --border-dark: rgba(26, 26, 26, 0.18);
            --text-primary: #1a1a1a;
            --text-secondary: #5c5c5c;
            --accent-color: #9b2d30;
            --shadow-color: rgba(26, 26, 26, 0.08);
            --font-mono: "Songti SC", "STSong", "Noto Serif SC", Georgia, serif;
            --content-font-family: var(--font-mono);
            --radius: 2px;
            --bubble-received-bg: rgba(255, 255, 255, 0.58);
            --panel-border: 1px solid rgba(26, 26, 26, 0.14);
            --panel-radius: 2px;
            --panel-shadow: 0 10px 28px rgba(26, 26, 26, 0.06);
            --input-border: 1px solid rgba(26, 26, 26, 0.14);
            --input-radius: 2px;
            --input-shadow: none;
            --button-border: 1px solid rgba(26, 26, 26, 0.18);
            --button-radius: 2px;
            --button-shadow: none;
            --send-button-background: #9b2d30;
            --send-button-color: #f7f4ee;
            --send-button-border: 1px solid #7f2427;
            --send-button-shadow: none;
            "#,
            r#"
            --bg-body: #121411;
            --bg-panel: rgba(28, 31, 29, 0.96);
            --bg-input: rgba(22, 25, 23, 0.98);
            --bg-button: transparent;
            --border-dark: rgba(228, 221, 209, 0.14);
            --text-primary: #e4ddd1;
            --text-secondary: #a39c90;
            --accent-color: #a84848;
            --shadow-color: rgba(0, 0, 0, 0.3);
            --bubble-received-bg: rgba(28, 31, 29, 0.96);
            --panel-border: 1px solid rgba(228, 221, 209, 0.1);
            --input-border: 1px solid rgba(228, 221, 209, 0.1);
            --button-border: 1px solid rgba(228, 221, 209, 0.12);
            --send-button-background: #a84848;
            --send-button-color: #e4ddd1;
            --send-button-border: 1px solid #8c3a3a;
            "#,
            r#"
            body.theme-ink {{
                background:
                    radial-gradient(ellipse 120% 70% at 50% -10%, rgba(255, 255, 255, 0.2), transparent 58%),
                    var(--bg-body);
            }}
            .theme-ink header, .theme-ink footer {{
                border-color: var(--border-dark);
            }}
            .theme-ink .bubble {{
                box-shadow: none;
            }}
            "#,
        ),
        _ => (
            r#"
            --bg-body: #dcdcdc;
            --bg-panel: #f3f3f3;
            --bg-input: #ffffff;
            --bg-button: #e0e0e0;
            --border-dark: #373737;
            --text-primary: #373737;
            --text-secondary: #707070;
            --accent-color: #487bdb;
            --shadow-color: #373737;
            --font-mono: "Courier New", Courier, monospace;
            --content-font-family: var(--font-mono);
            --radius: 0px;
            --bubble-received-bg: #ffffff;
            --panel-border: 2px solid var(--border-dark);
            --panel-radius: 4px;
            --panel-shadow: 2px 2px 0 0 var(--shadow-color);
            --input-border: 3px solid var(--border-dark);
            --input-radius: 0;
            --input-shadow: inset 4px 4px 0 rgba(0, 0, 0, 0.1);
            --button-border: 2px solid var(--border-dark);
            --button-radius: 0;
            --button-shadow: 2px 2px 0 0 var(--shadow-color);
            --button-active-transform: translate(2px, 2px);
            --button-active-shadow: 0 0 0 0 var(--shadow-color);
            --button-active-filled-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.2);
            --send-button-background: var(--accent-color);
            --send-button-color: #ffffff;
            --send-button-border: 2px solid var(--border-dark);
            --send-button-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.2);
            "#,
            r#"
            --bg-body: #121212;
            --bg-panel: #1e1e1e;
            --bg-input: #202020;
            --bg-button: #333333;
            --border-dark: #000000;
            --text-primary: #e0e0e0;
            --text-secondary: #a0a0a0;
            --accent-color: #5a8dee;
            --shadow-color: #000000;
            --bubble-received-bg: #1e1e1e;
            --panel-border: 2px solid #000000;
            --panel-shadow: 2px 2px 0 0 #000000;
            --input-border: 2px solid #000000;
            --input-shadow: inset 2px 2px 0 0 rgba(0,0,0,0.5);
            --button-border: 2px solid #000000;
            --button-shadow: 2px 2px 0 0 #000000;
            --send-button-background: #000000;
            --send-button-color: #ffffff;
            --send-button-border: 2px solid #000000;
            --send-button-shadow: none;
            "#,
            "",
        ),
    }
}

fn file_transfer_theme_css(theme: &str, color_mode: &str) -> String {
    let (light, dark, extra) = file_transfer_theme_variants(theme);
    // Theme snippets are kept next to Rust's `format!`-heavy HTML template,
    // where doubled braces are easier to maintain. Normalize them before the
    // CSS is sent to the browser.
    let extra = extra.replace("{{", "{").replace("}}", "}");
    let dark_css = match color_mode {
        "dark" => format!(":root {{{dark}}}"),
        "system" => format!("@media (prefers-color-scheme: dark) {{ :root {{{dark}}} }}"),
        _ => String::new(),
    };

    format!(":root {{{light}}}\n{dark_css}\n{extra}")
}

pub fn render_index(theme: &str, color_mode: &str, logo_base64: &str) -> String {
    let theme_css = file_transfer_theme_css(theme, color_mode);
    let mode_class = match color_mode {
        "dark" => "dark-mode",
        "light" => "light-mode",
        _ => "",
    };

    format!(
        r#"
<!DOCTYPE html>
<html lang="zh">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover, interactive-widget=resizes-content">
    <title>TieZ 终端传输</title>
    <style>
        * {{ box-sizing: border-box; -webkit-tap-highlight-color: transparent; }}
        
        :root {{
            --bg-body: #dcdcdc;
            --bg-panel: #f3f3f3;
            --border-dark: #373737;
            --text-primary: #373737;
            --accent-color: #487bdb;
            --shadow-color: #373737;
            --font-mono: "Courier New", Courier, monospace;
            --radius: 0px;
            --bubble-received-bg: #ffffff;
            --app-height: 100vh;
        }}

        @media (prefers-color-scheme: dark) {{
            .theme-mica, .theme-acrylic {{
                --bg-body: #101010;
                --bg-panel: #1c1c1c;
                --border-dark: rgba(255,255,255,0.08); /* Fainter border */
                --text-primary: #e0e0e0;
                --shadow-color: rgba(0,0,0,0.5);
            }}
            :root:not(.theme-mica):not(.theme-acrylic) {{
                --bg-body: #121212; /* Slightly cooler black */
                --bg-panel: #1e1e1e;
                --border-dark: #2a2a2a; /* Dark gray instead of pure black for borders */
                --text-primary: #e0e0e0; 
                --shadow-color: #000000;
                --bubble-received-bg: var(--bg-panel);
            }}
        }}

        /* Theme mica / acrylic overrides */
        .theme-mica, .theme-acrylic {{
            --bg-body: #f3f3f3;
            --bg-panel: #ffffff;
            --border-dark: rgba(0,0,0,0.1);
            --text-primary: #333;
            --shadow-color: rgba(0,0,0,0.1);
            --font-mono: "Segoe UI", system-ui, -apple-system, sans-serif;
            --radius: 12px;
            --bubble-received-bg: rgba(255, 255, 255, 0.9);
        }}

        @media (prefers-color-scheme: dark) {{
            .theme-mica, .theme-acrylic {{
                --bg-body: #1a1a1a;
                --bg-panel: #2a2a2a;
                --text-primary: #e0e0e0;
                --bubble-received-bg: rgba(40, 40, 40, 0.9);
            }}
        }}

        html, body {{
            height: 100%;
            width: 100%;
            margin: 0;
            padding: 0;
            overflow: hidden;
            position: fixed;
            top: 0;
            left: 0;
        }}

        body {{
            background-color: var(--bg-body);
            color: var(--text-primary);
            font-family: var(--content-font-family, var(--font-mono));
            display: flex;
            flex-direction: column;
            height: var(--app-height);
            transition: background 0.3s;
        }}

        header {{
            height: 60px;
            background: var(--bg-panel);
            border-bottom: 2px solid var(--border-dark);
            display: flex; align-items: center; justify-content: center;
            padding: 0 16px; position: relative;
            flex-shrink: 0;
            z-index: 10;
        }}
        .theme-mica header, .theme-acrylic header {{
            background: rgba(255,255,255,0.7); backdrop-filter: blur(20px);
            border-bottom: 1px solid rgba(0,0,0,0.1);
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-mica header, .theme-acrylic header {{ background: rgba(30,30,30,0.7); }}
        }}

        h1 {{ font-size: 18px; font-weight: 900; margin: 0; letter-spacing: -0.5px; }}
        
        .header-status {{
            position: absolute; left: 16px; top: 50%; transform: translateY(-50%);
            display: flex; align-items: center; gap: 6px; font-size: 12px; font-weight: bold;
        }}
        .status-dot {{ width: 8px; height: 8px; background: #4caf50; border-radius: 50%; box-shadow: 0 0 5px #4caf50; }}

        #chat-box {{
            flex: 1; overflow-y: auto; padding: 16px;
            display: flex; flex-direction: column; gap: 16px;
            scroll-behavior: smooth;
            padding-bottom: 40px;
        }}

        .timestamp {{
            font-size: 11px; text-align: center; opacity: 0.6;
            margin: 8px 0; font-weight: bold;
        }}

        .message {{ display: flex; gap: 10px; max-width: 90%; }}
        .message.received {{ align-self: flex-start; }}
        .message.sent {{ align-self: flex-end; flex-direction: row-reverse; }}

        .avatar {{
            width: 36px; height: 36px; background: #fff;
            border: 2px solid var(--border-dark);
            display: flex; align-items: center; justify-content: center;
            font-weight: bold; font-size: 18px; flex-shrink: 0;
            box-shadow: 2px 2px 0 var(--shadow-color);
            border-radius: var(--radius); overflow: hidden;
        }}
        .avatar img {{ width: 100%; height: 100%; object-fit: cover; }}
        
        .theme-mica .avatar, .theme-acrylic .avatar {{
            box-shadow: none; border-width: 1px;
        }}

        .bubble {{
            padding: 10px 14px;
            background: #fff;
            border: 2px solid var(--border-dark);
            box-shadow: 3px 3px 0 var(--shadow-color);
            font-size: 14px; line-height: 1.5;
            word-break: break-all;
            position: relative;
            border-radius: var(--radius);
        }}
        .message.received .bubble {{ background: var(--bubble-received-bg); }}
        .message.sent .bubble {{ background: var(--accent-color); color: #fff; border-color: var(--border-dark); }}
        
        /* Mica Style Bubbles */
        .theme-mica .bubble, .theme-acrylic .bubble {{
            box-shadow: 0 2px 10px var(--shadow-color);
            border: 1px solid var(--border-dark);
            border-radius: 12px;
        }}
        .theme-mica .message.received .bubble, .theme-acrylic .message.received .bubble {{
             background: var(--bubble-received-bg);
             backdrop-filter: blur(10px);
        }}

        /* Triangle tail for retro style only */
        :root:not(.theme-mica):not(.theme-acrylic) .message.received .bubble::after {{
            content: ''; position: absolute; left: -10px; top: 10px;
            width: 0; height: 0; border: 5px solid transparent;
            border-right-color: var(--bubble-received-bg);
        }}
        :root:not(.theme-mica):not(.theme-acrylic) .message.received .bubble::before {{
            content: ''; position: absolute; left: -13px; top: 9px;
            width: 0; height: 0; border: 6px solid transparent;
            border-right-color: var(--border-dark);
        }}
        :root:not(.theme-mica):not(.theme-acrylic) .message.sent .bubble::after {{
            content: ''; position: absolute; right: -10px; top: 10px;
            width: 0; height: 0; border: 5px solid transparent;
            border-left-color: var(--accent-color);
        }}
        :root:not(.theme-mica):not(.theme-acrylic) .message.sent .bubble::before {{
            content: ''; position: absolute; right: -13px; top: 9px;
            width: 0; height: 0; border: 6px solid transparent;
            border-left-color: var(--border-dark);
        }}
        
        /* Mica doesn't use these triangles */
        .theme-mica .message.received .bubble::before,
        .theme-acrylic .message.received .bubble::before {{ border-right-color: var(--border-dark); }}
        .theme-mica .message.received .bubble::after,
        .theme-acrylic .message.received .bubble::after {{ border-right-color: var(--bubble-received-bg); }}
        
        @media (prefers-color-scheme: dark) {{
            .theme-mica .message.received .bubble, .theme-acrylic .message.received .bubble {{ background: var(--bubble-received-bg); }}
        }}

        .theme-mica .message.sent .bubble::before,
        .theme-acrylic .message.sent .bubble::before {{ display: none; }}
        .theme-mica .message.sent .bubble::after,
        .theme-acrylic .message.sent .bubble::after {{ 
            border-left-color: var(--accent-color) !important; 
            right: -7px; 
            bottom: 10px;
        }}

        
        /* File Card */
        .file-card {{ display: flex; align-items: center; gap: 12px; }}
        .file-icon {{ font-size: 24px; flex-shrink: 0; }}
        .file-info {{ display: flex; flex-direction: column; min-width: 0; overflow: hidden; }}
        .file-name {{ font-weight: 700; font-family: var(--font-mono); font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 160px; }}
        .file-size {{ font-size: 11px; font-family: var(--font-mono); opacity: 0.8; margin-top: 2px; }}
        .batch-bubble {{ width: min(310px, calc(100vw - 72px)); padding: 9px; }}
        .batch-header {{ display:flex; align-items:center; gap:10px; padding:5px; cursor:pointer; }}
        .batch-header-icon {{ display:grid; width:38px; height:38px; place-items:center; flex:0 0 38px; border-radius:var(--input-radius, 8px); background:rgba(127,127,127,0.12); font-size:22px; }}
        .batch-header-copy {{ display:flex; min-width:0; flex:1; flex-direction:column; }}
        .batch-title {{ overflow:hidden; font-weight:800; text-overflow:ellipsis; white-space:nowrap; }}
        .batch-meta, .batch-file-status {{ font-size:10px; opacity:0.7; }}
        .batch-toggle {{ flex:0 0 auto; font-size:15px; opacity:0.65; }}
        .batch-files {{ display:none; max-height:230px; margin-top:7px; padding:5px 0; overflow-y:auto; border-top:1px solid var(--border-dark); border-bottom:1px solid var(--border-dark); }}
        .batch-bubble.expanded .batch-files {{ display:block; }}
        .batch-file-row {{ display:flex; align-items:center; gap:9px; min-height:40px; padding:5px; color:inherit; text-decoration:none; border-radius:var(--input-radius, 6px); }}
        .batch-file-row:active {{ background:rgba(127,127,127,0.12); }}
        .batch-file-icon {{ flex:0 0 auto; font-size:19px; }}
        .batch-file-copy {{ display:flex; min-width:0; flex:1; flex-direction:column; }}
        .batch-file-name {{ overflow:hidden; font-size:12px; font-weight:700; text-overflow:ellipsis; white-space:nowrap; }}
        .batch-actions {{ display:flex; align-items:center; justify-content:space-between; gap:8px; min-height:32px; padding:6px 4px 0; }}
        .batch-download {{ min-height:28px; padding:4px 9px; border:var(--button-border, 1px solid var(--border-dark)); border-radius:var(--button-radius, 6px); color:var(--send-button-color, #fff); background:var(--send-button-background, var(--accent-color)); font:700 11px/1 var(--font-mono); }}
        .batch-download:disabled {{ opacity:0.55; }}

        /* Image Preview */
        .img-preview {{
            max-width: 100%;
            width: auto;
            max-height: 400px;
            display: block;
            border: 1px solid rgba(0,0,0,0.1);
            margin-bottom: 6px;
            border-radius: 8px;
            object-fit: contain;
        }}
        .video-preview {{
            width: 100%;
            max-width: 100%;
            min-height: 120px;
            max-height: 400px;
            display: block;
            margin-bottom: 6px;
            border-radius: 8px;
            background: #000;
        }}

        .progress-wrapper {{ margin-top: 8px; border-top: 1px dashed rgba(255,255,255,0.3); padding-top: 4px; }}
        .progress-bar {{ width: 100%; height: 4px; background: rgba(0,0,0,0.1); border-radius: 2px; overflow: hidden; margin-top: 4px; }}
        .progress-inner {{ height: 100%; background: var(--accent-color); width: 0%; transition: width 0.2s; }}

        footer {{
            padding: 10px 16px;
            padding-bottom: calc(10px + env(safe-area-inset-bottom));
            background: var(--bg-panel);
            border-top: 2px solid var(--border-dark);
            display: flex; gap: 12px; align-items: flex-end;
            flex-shrink: 0;
            z-index: 10;
        }}
        .theme-mica footer, .theme-acrylic footer {{
            background: rgba(255,255,255,0.7); backdrop-filter: blur(20px);
            border-top: 1px solid rgba(0,0,0,0.1);
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-mica footer, .theme-acrylic footer {{ background: rgba(30,30,30,0.7); }}
        }}
        
        .retro-btn {{
            background: var(--bg-button);
            border: var(--button-border, 2px solid var(--border-dark));
            box-shadow: var(--button-shadow, 2px 2px 0 0 var(--shadow-color));
            display: flex; align-items: center; justify-content: center;
            cursor: pointer; color: var(--text-primary);
            transition: all 0.1s;
            height: 40px;
            flex-shrink: 0;
            border-radius: var(--button-radius, var(--radius));
        }}
        .retro-btn:active {{
            transform: var(--button-active-transform, translate(2px, 2px));
            box-shadow: var(--button-active-shadow, 0 0 0 0 var(--shadow-color));
        }}
        
        .theme-mica .retro-btn, .theme-acrylic .retro-btn {{
            box-shadow: none; border-width: 1px;
            background: rgba(255,255,255,0.5);
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-mica .retro-btn, .theme-acrylic .retro-btn {{ background: rgba(255,255,255,0.1); }}
        }}

        .file-send-control {{
            position: relative; display: flex; height: 40px; flex-shrink: 0;
            border-radius: var(--button-radius, var(--radius));
            box-shadow: var(--button-shadow, 2px 2px 0 0 var(--shadow-color));
        }}
        .file-send-control .retro-btn {{ box-shadow: none; }}
        .add-btn {{
            width: 40px; font-size: 24px; font-weight: 900;
            border-radius: var(--button-radius, var(--radius)) 0 0 var(--button-radius, var(--radius));
        }}
        .file-mode-toggle {{
            width: 24px; padding: 0; margin-left: -2px; font-size: 12px;
            border-radius: 0 var(--button-radius, var(--radius)) var(--button-radius, var(--radius)) 0;
            color: var(--accent-color);
        }}
        .file-mode-menu {{
            position: absolute; left: 0; bottom: calc(100% + 10px); z-index: 50;
            display: none; width: 230px; padding: 6px;
            border: var(--button-border, 2px solid var(--border-dark));
            border-radius: var(--button-radius, var(--radius));
            background: var(--bg-panel); box-shadow: var(--button-shadow, 3px 3px 0 var(--shadow-color));
        }}
        .file-mode-menu.open {{ display: grid; gap: 4px; }}
        .file-mode-option {{
            display: grid; grid-template-columns: 26px 1fr 18px; align-items: center;
            gap: 8px; width: 100%; padding: 9px; border: 0;
            border-radius: max(4px, var(--radius)); color: var(--text-primary);
            background: transparent; text-align: left; font-family: inherit;
        }}
        .file-mode-option.active {{ background: color-mix(in srgb, var(--accent-color) 15%, transparent); }}
        .file-mode-option strong, .file-mode-option small {{ display: block; }}
        .file-mode-option small {{ margin-top: 2px; color: var(--text-secondary); font-size: 10px; }}
        .file-mode-check {{ color: var(--accent-color); font-weight: 900; }}
        .send-btn {{
            min-width: 72px;
            padding: 0 16px;
            background: var(--send-button-background, var(--accent-color));
            color: var(--send-button-color, #ffffff);
            border: var(--send-button-border, var(--button-border, 2px solid var(--border-dark)));
            box-shadow: var(--send-button-shadow, var(--button-active-filled-shadow, 2px 2px 0 0 var(--shadow-color)));
        }}
        
        .text-input {{ 
            flex: 1; height: 40px; min-height: 40px; max-height: 100px; padding: 10px 12px;
            background: var(--bg-input); border: var(--input-border, 2px solid var(--border-dark));
            box-shadow: var(--input-shadow, inset 2px 2px 0 rgba(0,0,0,0.1));
            font-size: 14px; font-family: var(--content-font-family, var(--font-mono));
            color: var(--text-primary); outline: none;
            border-radius: var(--input-radius, var(--radius)); -webkit-appearance: none;
            resize: none; overflow-y: auto; line-height: 20px;
        }}
        .theme-mica .text-input, .theme-acrylic .text-input {{ box-shadow: none; border-width: 1px; }}
        @media (prefers-color-scheme: dark) {{
            .theme-mica .text-input, .theme-acrylic .text-input {{ background: rgba(255,255,255,0.05); color: #fff; }}

            /* Retro Dark Mode Overrides */
            :root:not(.theme-mica):not(.theme-acrylic) .retro-btn {{
                background: #333;
                border-color: #000;
                color: #e0e0e0;
                box-shadow: 2px 2px 0 0 #000;
            }}
            :root:not(.theme-mica):not(.theme-acrylic) .retro-btn:active {{
                box-shadow: none;
                transform: translate(2px, 2px);
            }}
            :root:not(.theme-mica):not(.theme-acrylic) .retro-btn.send-btn {{
                background: #000;
                color: #fff;
                border-color: #000;
            }}
            :root:not(.theme-mica):not(.theme-acrylic) .text-input {{
                background: #202020;
                color: #e0e0e0;
                border-color: #000;
                box-shadow: inset 2px 2px 0 0 rgba(0,0,0,0.5);
            }}
        }}
        
        .expand-btn {{
            width: 30px; height: 30px; display: none; align-items: center; justify-content: center;
            position: absolute; right: 5px; bottom: 5px;
            background: var(--bg-body); border: 1px solid var(--border-dark);
            border-radius: 4px; cursor: pointer; z-index: 5;
            color: var(--text-primary);
        }}

        /* Fullscreen Editor */
        #fs-editor {{
            position: fixed; top: 0; left: 0; width: 100%; height: 100%;
            background: var(--bg-body); z-index: 1000;
            display: none; flex-direction: column;
            padding: 16px;
        }}
        .theme-mica #fs-editor, .theme-acrylic #fs-editor {{
             background: rgba(255,255,255,0.95); backdrop-filter: blur(20px);
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-mica #fs-editor, .theme-acrylic #fs-editor {{ background: rgba(20,20,20,0.95); }}
            
            /* Retro Dark Mode Overrides for Fullscreen Editor */
            :root:not(.theme-mica):not(.theme-acrylic) #fs-textarea {{
                background: #202020;
                color: #e0e0e0;
                border: 2px solid #000;
            }}
        }}

        #fs-textarea {{
            flex: 1; width: 100%; border: 2px solid var(--border-dark);
            padding: 16px; font-size: 16px; font-family: var(--font-mono);
            background: #fff; color: var(--text-primary); margin-bottom: 16px;
            border-radius: var(--radius); resize: none; outline: none;
        }}
        .theme-mica #fs-textarea, .theme-acrylic #fs-textarea {{
            background: rgba(255,255,255,0.5); border-width: 1px; box-shadow: none;
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-mica #fs-textarea, .theme-acrylic #fs-textarea {{ background: rgba(255,255,255,0.05); color: #fff; }}
        }}

        .fs-toolbar {{ display: flex; justify-content: flex-end; gap: 12px; }}
        {theme_css}
    </style>
</head>
<body class="theme-{theme} {mode_class}">
    <header>
        <div class="header-status">
            <div class="status-dot"></div>
            <span id="device-count">Linked</span>
        </div>
        <h1>TieZ 终端</h1>
    </header>

    <div id="chat-box">
        <div class="timestamp">SYS: <span id="time-now"></span></div>
        <div class="message received">
            <div class="avatar"><img src="{logo_base64}" onerror="this.innerText='T'"></div>
            <div class="bubble">
                <div style="font-weight:900; margin-bottom:4px">SYSTEM READY</div>
                发送文字、图片或文件到电脑。
            </div>
        </div>
    </div>
    
    <!-- Full Screen Editor Modal -->
    <div id="fs-editor">
        <div style="font-weight:bold; margin-bottom:8px; display:flex; justify-content:space-between; align-items:center">
            <span>FULL SCREEN EDIT</span>
            <span onclick="closeFullscreen()" style="cursor:pointer; padding:4px;">✕</span>
        </div>
        <textarea id="fs-textarea" placeholder="输入内容..."></textarea>
        <div class="fs-toolbar">
            <button class="retro-btn" onclick="closeFullscreen()">CANCEL</button>
            <button class="retro-btn send-btn" onclick="sendFullscreen()">SEND</button>
        </div>
    </div>

    <footer>
        <div class="file-send-control" id="file-send-control">
            <label for="file-input" class="retro-btn add-btn" aria-label="发送文件">+</label>
            <button type="button" class="retro-btn file-mode-toggle" id="file-mode-toggle" aria-label="选择多文件发送方式" aria-expanded="false">⌃</button>
            <div class="file-mode-menu" id="file-mode-menu">
                <button type="button" class="file-mode-option" data-mode="package">
                    <span>▣</span><span><strong>作为文件包发送</strong><small>折叠显示，可下载 ZIP</small></span><span class="file-mode-check"></span>
                </button>
                <button type="button" class="file-mode-option" data-mode="separate">
                    <span>▤</span><span><strong>分别发送</strong><small>每个文件独立显示</small></span><span class="file-mode-check"></span>
                </button>
            </div>
        </div>
        <div style="position:relative; flex:1; display:flex;">
            <textarea class="text-input" id="text-input" placeholder="输入文字..." rows="1"></textarea>
            <div class="expand-btn" id="expand-btn" onclick="openFullscreen()">⤢</div>
        </div>
        <button class="retro-btn send-btn" id="send-btn">SEND</button>
        <input type="file" id="file-input" multiple style="display:none">
    </footer>

    <!-- Fullscreen Image Overlay -->
    <div id="img-overlay" style="display:none; position:fixed; top:0; left:0; width:100%; height:100%; background:rgba(0,0,0,0.9); backdrop-filter:blur(5px); z-index:9999; align-items:center; justify-content:center; flex-direction:column;">
        <div style="position:absolute; top:20px; right:20px; color:white; font-size:24px; cursor:pointer; padding:10px;" onclick="closeOverlay()">✕</div>
        <img id="overlay-img" style="max-width:95%; max-height:90%; object-fit:contain; border-radius:4px; box-shadow:0 0 20px rgba(0,0,0,0.5);">
    </div>

    <script>
        // The one-time QR credential is persisted as an HttpOnly session cookie by
        // the server. Remove it from the visible URL and browser history immediately.
        if (window.location.search.includes('auth=')) {{
            window.history.replaceState(null, '', window.location.pathname);
        }}
        const fileInput = document.getElementById('file-input');
        const textInput = document.getElementById('text-input');
        const sendBtn = document.getElementById('send-btn');
        const chatBox = document.getElementById('chat-box');
        const fileModeToggle = document.getElementById('file-mode-toggle');
        const fileModeMenu = document.getElementById('file-mode-menu');
        const fileSendControl = document.getElementById('file-send-control');
        let multiFileSendMode = localStorage.getItem('tiez.file-transfer.multi-file-send-mode') === 'separate'
            ? 'separate' : 'package';

        function renderFileMode() {{
            fileModeMenu.querySelectorAll('.file-mode-option').forEach(option => {{
                const active = option.dataset.mode === multiFileSendMode;
                option.classList.toggle('active', active);
                option.querySelector('.file-mode-check').textContent = active ? '✓' : '';
            }});
        }}
        renderFileMode();
        fileModeToggle.onclick = () => {{
            const open = !fileModeMenu.classList.contains('open');
            fileModeMenu.classList.toggle('open', open);
            fileModeToggle.setAttribute('aria-expanded', String(open));
        }};
        fileModeMenu.querySelectorAll('.file-mode-option').forEach(option => {{
            option.onclick = () => {{
                multiFileSendMode = option.dataset.mode;
                localStorage.setItem('tiez.file-transfer.multi-file-send-mode', multiFileSendMode);
                renderFileMode();
                fileModeMenu.classList.remove('open');
                fileModeToggle.setAttribute('aria-expanded', 'false');
                fileInput.click();
            }};
        }});
        document.addEventListener('pointerdown', event => {{
            if (!fileSendControl.contains(event.target)) {{
                fileModeMenu.classList.remove('open');
                fileModeToggle.setAttribute('aria-expanded', 'false');
            }}
        }});
        
        const now = new Date();
        document.getElementById('time-now').innerText = `${{now.getHours().toString().padStart(2,'0')}}:${{now.getMinutes().toString().padStart(2,'0')}}`;
        
        let lastId = 0;
        let isUploading = false;
        const deviceId = localStorage.getItem('tiez_device_id') || ('m-' + Math.random().toString(36).substr(2, 9));
        localStorage.setItem('tiez_device_id', deviceId);
        
        const deviceName = "Mobile";
        const TIEZ_LOGO = "{logo_base64}";
        const pendingUploads = new Map(); // filename -> [elements]
        const batchElements = new Map();

        function scrollToBottom() {{
            chatBox.scrollTop = chatBox.scrollHeight;
        }}

        function syncViewportMetrics() {{
            const vv = window.visualViewport;
            if (!vv) {{
                document.documentElement.style.setProperty('--app-height', `${{window.innerHeight}}px`);
                return;
            }}

            // Pull the height from the visual viewport (excludes keyboard)
            document.documentElement.style.setProperty('--app-height', `${{vv.height}}px`);
            
            // Keep the fixed body aligned with the visual viewport's top/left
            document.body.style.transform = `translate(${{vv.offsetLeft}}px, ${{vv.offsetTop}}px)`;
            
            // Prevent the browser from scrolling the layout viewport away from origin
            if (window.scrollY !== 0 || window.scrollX !== 0) {{
                window.scrollTo(0, 0);
            }}
        }}

        if (window.visualViewport) {{
            window.visualViewport.addEventListener('resize', syncViewportMetrics);
            window.visualViewport.addEventListener('scroll', syncViewportMetrics);
        }}
        window.addEventListener('resize', syncViewportMetrics);
        // Also sync on focus/blur to handle various keyboard states
        document.addEventListener('focusin', () => setTimeout(syncViewportMetrics, 50));
        document.addEventListener('focusout', () => setTimeout(syncViewportMetrics, 50));
        syncViewportMetrics();

        function escapeHTML(str) {{
            if (!str) return '';
            return str.replace(/[&<>"']/g, function(m) {{
                return {{
                    '&': '&amp;',
                    '<': '&lt;',
                    '>': '&gt;',
                    '"': '&quot;',
                    "'": '&#39;'
                }}[m];
            }});
        }}

        function normalizeFileName(name) {{
            if (!name) return '';
            const base = name.split('/').pop().split('\\').pop();
            const m = base.match(/^\d{{8,}}_(.+)$/);
            return escapeHTML(m ? m[1] : base);
        }}
        function extractNameFromContent(content, file_path) {{
            if (content && content.startsWith('/download/') && content.includes('?name=')) {{
                const idx = content.indexOf('?name=');
                if (idx !== -1) {{
                    try {{ return decodeURIComponent(content.slice(idx + 6)); }} catch (e) {{ return content; }}
                }}
            }}
            return file_path || content || '';
        }}
        function addPendingUpload(fileName, el) {{
            const list = pendingUploads.get(fileName) || [];
            list.push(el);
            pendingUploads.set(fileName, list);
        }}
        function takePendingUpload(maybeName) {{
            for (const [name, list] of pendingUploads.entries()) {{
                if (maybeName.endsWith(name) && list.length > 0) {{
                    const el = list.shift();
                    if (list.length === 0) pendingUploads.delete(name);
                    return el;
                }}
            }}
            return null;
        }}
        function formatBytes(bytes) {{
            if (!bytes || bytes <= 0) return '';
            const units = ['B', 'KB', 'MB', 'GB', 'TB'];
            const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
            const value = bytes / Math.pow(1024, index);
            return `${{value >= 10 || index === 0 ? value.toFixed(0) : value.toFixed(1)}} ${{units[index]}}`;
        }}
        async function downloadBatch(batchId, button) {{
            if (button.disabled) return;
            button.disabled = true;
            const original = button.textContent;
            button.textContent = '正在打包…';
            try {{
                const response = await fetch('/download-batch/' + encodeURIComponent(batchId), {{ method: 'POST' }});
                if (!response.ok) throw new Error(await response.text());
                const result = await response.json();
                window.location.href = result.url;
                button.textContent = '已开始下载';
            }} catch (error) {{
                console.error(error);
                button.textContent = '打包失败';
            }} finally {{
                setTimeout(() => {{
                    button.disabled = false;
                    button.textContent = original;
                }}, 1800);
            }}
        }}
        function appendBatchMessage(msg, direction, senderName) {{
            const batchId = String(msg.batch_id || '');
            if (!batchId) return null;
            let entry = batchElements.get(batchId);
            if (!entry) {{
                const outer = document.createElement('div');
                outer.className = `message ${{direction}}`;
                if (direction === 'received') {{
                    const avatar = document.createElement('div');
                    avatar.className = 'avatar';
                    avatar.innerHTML = `<img src="${{TIEZ_LOGO}}" alt="TieZ">`;
                    outer.appendChild(avatar);
                }}
                const bubble = document.createElement('div');
                bubble.className = 'bubble batch-bubble';
                bubble.innerHTML = `
                    ${{senderName ? `<div style="font-size:10px;opacity:.65;margin:0 5px 3px">${{escapeHTML(senderName)}}</div>` : ''}}
                    <div class="batch-header">
                        <span class="batch-header-icon">🗂️</span>
                        <span class="batch-header-copy">
                            <span class="batch-title">${{escapeHTML(msg.batch_name || '文件包')}}</span>
                            <span class="batch-meta"><span class="batch-count">0</span>/${{Number(msg.batch_total || 0)}} 个文件${{msg.batch_size ? ` · ${{formatBytes(msg.batch_size)}}` : ''}}</span>
                        </span>
                        <span class="batch-toggle">›</span>
                    </div>
                    <div class="batch-files"></div>
                    <div class="batch-actions">
                        <span class="batch-file-status">文件独立传输</span>
                        <button type="button" class="batch-download">下载 ZIP</button>
                    </div>`;
                outer.appendChild(bubble);
                bubble.querySelector('.batch-header').onclick = () => {{
                    bubble.classList.toggle('expanded');
                    bubble.querySelector('.batch-toggle').textContent = bubble.classList.contains('expanded') ? '⌄' : '›';
                }};
                bubble.querySelector('.batch-download').onclick = event => {{
                    event.stopPropagation();
                    downloadBatch(batchId, event.currentTarget);
                }};
                chatBox.appendChild(outer);
                entry = {{ outer, bubble, files: bubble.querySelector('.batch-files'), indices: new Set() }};
                batchElements.set(batchId, entry);
            }}

            const index = Number(msg.batch_index || 0);
            let row = entry.files.querySelector(`[data-batch-index="${{index}}"]`);
            const rawName = extractNameFromContent(msg.content || '', msg.file_path);
            const fileName = normalizeFileName(rawName || `文件 ${{index + 1}}`);
            const url = msg.content && msg.content.startsWith('/download/') ? msg.content : '';
            if (!row) {{
                row = document.createElement(url ? 'a' : 'div');
                row.className = 'batch-file-row';
                row.dataset.batchIndex = String(index);
                if (url) row.href = url;
                row.innerHTML = `
                    <span class="batch-file-icon">📄</span>
                    <span class="batch-file-copy">
                        <span class="batch-file-name">${{fileName}}</span>
                        <span class="batch-file-status">${{formatBytes(msg.file_size) || '准备中…'}}</span>
                    </span>`;
                entry.files.appendChild(row);
                entry.indices.add(index);
            }} else {{
                if (url) {{
                    if (row.tagName === 'A') row.href = url;
                    else {{
                        row.style.cursor = 'pointer';
                        row.onclick = () => window.location.href = url;
                    }}
                }}
                row.querySelector('.batch-file-name').textContent = fileName;
                row.querySelector('.batch-file-status').textContent = formatBytes(msg.file_size) || '已完成';
            }}
            entry.bubble.querySelector('.batch-count').textContent = String(entry.indices.size);
            return row;
        }}
        function createMessageElement(direction, content, senderName, msgType, file_path) {{
            const div = document.createElement('div');
            div.className = `message ${{direction}}`;
            
            let bubbleContent = escapeHTML(content);
            if (msgType === 'image' || (content.match(/\.(jpg|jpeg|png|gif|webp)$/i) && file_path)) {{
                const useContent = content.startsWith('data:') || content.startsWith('/download/') || content.startsWith('http');
                const src = escapeHTML(useContent ? content : (file_path || content));
                bubbleContent = `<img src="${{src}}" class="img-preview" onclick="openOverlay('${{src}}')">`;
            }} else if (msgType === 'video') {{
                const useContent = content.startsWith('/download/') || content.startsWith('http');
                const src = escapeHTML(useContent ? content : (file_path || content));
                bubbleContent = `<video class="video-preview" controls src="${{src}}"></video>`;
            }} else if (msgType === 'file' || file_path) {{
                 const rawName = extractNameFromContent(content, file_path);
                 const fileName = normalizeFileName(rawName);
                 bubbleContent = `
                    <div class="file-card">
                        <div class="file-icon">📄</div>
                        <div class="file-info">
                            <span class="file-name">${{fileName}}</span>
                            <span class="file-size">DOWNLOAD</span>
                        </div>
                    </div>
                 `;
            }}

            const escapedSenderName = escapeHTML(senderName);
            div.innerHTML = `
                ${{direction === 'received' ? (() => {{
                    const name = (senderName || '').trim();
                    const lower = name.toLowerCase();
                    const isPc = name === '电脑' || name === 'PC' || lower === 'pc' || lower === 'tiez';
                    if (isPc) {{
                        return `<div class="avatar"><img src="${{TIEZ_LOGO}}" alt="TieZ"></div>`;
                    }}
                    return `<div class="avatar">${{name ? escapeHTML(name[0]) : '?'}}</div>`;
                }})() : ''}}
                <div class="bubble">
                    ${{escapedSenderName && escapedSenderName !== 'System' ? `<div style="font-size:10px; opacity:0.6; margin-bottom:2px">${{escapedSenderName}}</div>` : ''}}
                    ${{bubbleContent}}
                </div>
            `;
            
            const downloadUrl = content.startsWith('/download/') ? content : file_path;
            if (downloadUrl && msgType !== 'image' && msgType !== 'video') {{
                div.querySelector('.bubble').style.cursor = 'pointer';
                div.querySelector('.bubble').onclick = () => window.location.href = downloadUrl;
            }}
            
            return div;
        }}

        function openOverlay(src) {{
            document.getElementById('overlay-img').src = src;
            document.getElementById('img-overlay').style.display = 'flex';
        }}
        function closeOverlay() {{
            document.getElementById('img-overlay').style.display = 'none';
        }}

        function openFullscreen() {{
            document.getElementById('fs-textarea').value = textInput.value;
            document.getElementById('fs-editor').style.display = 'flex';
            document.getElementById('fs-textarea').focus();
        }}
        function closeFullscreen() {{
            document.getElementById('fs-editor').style.display = 'none';
        }}
        function sendFullscreen() {{
            const val = document.getElementById('fs-textarea').value;
            if (val.trim()) {{
                textInput.value = val;
                sendBtn.click();
            }}
            closeFullscreen();
        }}

        // Adjust textarea height
        textInput.addEventListener('input', function() {{
            this.style.height = '40px';
            const newHeight = Math.min(this.scrollHeight, 100);
            this.style.height = newHeight + 'px';
            document.getElementById('expand-btn').style.display = newHeight > 50 ? 'flex' : 'none';
        }});

        textInput.addEventListener('focus', () => {{
            setTimeout(() => {{
                syncViewportMetrics();
                scrollToBottom();
            }}, 80);
        }});

        window.addEventListener('resize', syncViewportMetrics);
        window.addEventListener('orientationchange', syncViewportMetrics);
        if (window.visualViewport) {{
            window.visualViewport.addEventListener('resize', syncViewportMetrics);
            window.visualViewport.addEventListener('scroll', syncViewportMetrics);
        }}
        syncViewportMetrics();

        // WebSocket Setup
        let socket;
        function connectWS() {{
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            socket = new WebSocket(`${{protocol}}//${{window.location.host}}/ws`);
            
            socket.onopen = () => {{
                socket.send(JSON.stringify({{ type: 'identity', device_id: deviceId, device_name: deviceName }}));
                console.log('WS Connected');
            }};
            
            socket.onmessage = (e) => {{
                const msg = JSON.parse(e.data);
                if (msg.batch_id) {{
                    if (msg.direction === 'in' && msg.sender_id === deviceId) {{
                        const rawName = extractNameFromContent(msg.content, msg.file_path);
                        const pending = takePendingUpload(normalizeFileName(rawName));
                        if (pending) pending.remove();
                    }}
                    appendBatchMessage(
                        msg,
                        msg.direction === 'out' ? 'received' : 'sent',
                        msg.direction === 'out' ? msg.sender_name : 'You'
                    );
                    scrollToBottom();
                    return;
                }}
                if (msg.direction === 'in' && msg.sender_id === deviceId && (msg.msg_type === 'file' || msg.msg_type === 'image' || msg.msg_type === 'video')) {{
                    const rawName = extractNameFromContent(msg.content, msg.file_path);
                    const maybeName = normalizeFileName(rawName);
                    const pending = takePendingUpload(maybeName);
                    if (pending) {{
                        const replacement = createMessageElement('sent', msg.content, 'You', msg.msg_type, msg.file_path);
                        pending.replaceWith(replacement);
                        scrollToBottom();
                        return;
                    }}
                }}
                if (msg.direction === 'out') {{
                    const el = createMessageElement('received', msg.content, msg.sender_name, msg.msg_type, msg.file_path);
                    chatBox.appendChild(el);
                    scrollToBottom();
                }} else if (msg.direction === 'in') {{
                    const el = createMessageElement('sent', msg.content, 'You', msg.msg_type, msg.file_path);
                    chatBox.appendChild(el);
                    scrollToBottom();
                }}
            }};
            
            socket.onclose = () => setTimeout(connectWS, 3000);
        }}
        connectWS();

        sendBtn.onclick = async () => {{
            const text = textInput.value.trim();
            if (!text || isUploading) return;
            
            textInput.value = '';
            textInput.style.height = '40px';
            document.getElementById('expand-btn').style.display = 'none';

            try {{
                await fetch('/send-text', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ content: text, sender_id: deviceId, sender_name: deviceName }})
                }});
            }} catch(e) {{ alert('Send failed'); }}
        }};

        fileInput.onchange = async () => {{
            if (!fileInput.files.length || isUploading) return;
            const files = Array.from(fileInput.files);
            fileInput.value = '';
            const batch = files.length > 1 && multiFileSendMode === 'package' ? {{
                id: 'mobile_' + Date.now() + '_' + Math.random().toString(36).slice(2, 8),
                name: files.length + ' 个文件',
                total: files.length,
                totalSize: files.reduce((sum, file) => sum + file.size, 0)
            }} : null;
            for(let index = 0; index < files.length; index++) {{
                await uploadFile(files[index], batch ? {{ ...batch, index }} : null);
            }}
        }};

        async function uploadFile(file, batch) {{
            isUploading = true;
            const isImage = file.type.startsWith('image/');
            const isVideo = file.type.startsWith('video/');
            const msgType = isVideo ? 'video' : (isImage ? 'image' : 'file');
            const previewUrl = (isImage || isVideo) ? URL.createObjectURL(file) : file.name;
            const pendingMessage = batch ? {{
                batch_id: batch.id,
                batch_name: batch.name,
                batch_index: batch.index,
                batch_total: batch.total,
                batch_size: batch.totalSize,
                file_size: file.size,
                content: file.name,
                msg_type: msgType
            }} : null;
            const batchRow = pendingMessage ? appendBatchMessage(pendingMessage, 'sent', 'You') : null;
            const el = batchRow ? batchRow.closest('.message') : createMessageElement('sent', previewUrl, 'You', msgType, undefined);
            el.dataset.fileName = file.name;
            el.dataset.pending = 'true';
            if (!batchRow) addPendingUpload(file.name, el);
            const progressWrapper = document.createElement('div');
            progressWrapper.className = 'progress-wrapper';
            progressWrapper.innerHTML = `<div style="font-size:10px">0%</div><div class="progress-bar"><div class="progress-inner"></div></div>`;
            if (!batchRow) {{
                el.querySelector('.bubble').appendChild(progressWrapper);
                chatBox.appendChild(el);
            }}
            scrollToBottom();

            const CHUNK_SIZE = 1024 * 512; // 512KB
            const totalChunks = Math.max(1, Math.ceil(file.size / CHUNK_SIZE));
            const uploadId = Math.random().toString(36).substr(2, 9);
            let uploadFailed = false;
            let uploadError = '';

            for (let i = 0; i < totalChunks; i++) {{
                const start = i * CHUNK_SIZE;
                const end = Math.min(file.size, start + CHUNK_SIZE);
                const chunk = file.slice(start, end);

                const formData = new FormData();
                formData.append('file', chunk);
                formData.append('metadata', JSON.stringify({{
                    upload_id: uploadId,
                    chunk_index: i,
                    total_chunks: totalChunks,
                    file_name: file.name,
                    sender_id: deviceId,
                    sender_name: deviceName,
                    total_size: file.size,
                    content_type: file.type,
                    batch_id: batch?.id,
                    batch_name: batch?.name,
                    batch_index: batch?.index,
                    batch_total: batch?.total,
                    batch_size: batch?.totalSize
                }}));

                try {{
                    let res = null;
                    let lastError = null;
                    for (let attempt = 0; attempt < 3; attempt++) {{
                        try {{
                            res = await fetch('/upload-chunk', {{ method: 'POST', body: formData }});
                            if (res.ok) break;
                            const detail = await res.text().catch(() => '');
                            lastError = new Error(detail || `HTTP ${{res.status}}`);
                            // Validation and storage errors will not improve by retrying.
                            if (res.status >= 400 && res.status < 500 && res.status !== 408 && res.status !== 429) break;
                        }} catch (error) {{
                            lastError = error;
                        }}
                        if (attempt < 2) await new Promise(resolve => setTimeout(resolve, 500 * (attempt + 1)));
                    }}
                    if (!res || !res.ok) throw lastError || new Error('网络连接已中断');
                    
                    const percent = Math.round(((i + 1) / totalChunks) * 100);
                    if (batchRow) {{
                        batchRow.querySelector('.batch-file-status').textContent = `${{formatBytes(file.size)}} · ${{percent}}%`;
                    }} else {{
                        progressWrapper.querySelector('.progress-inner').style.width = percent + '%';
                        progressWrapper.querySelector('div').innerText = percent + '%';
                    }}
                }} catch (e) {{
                    uploadError = e instanceof Error ? e.message : String(e || '');
                    uploadFailed = true;
                    if (batchRow) {{
                        batchRow.querySelector('.batch-file-status').textContent = '上传失败';
                    }} else {{
                        el.dataset.pending = 'false';
                        const list = pendingUploads.get(file.name) || [];
                        const idx = list.indexOf(el);
                        if (idx >= 0) list.splice(idx, 1);
                        if (list.length === 0) pendingUploads.delete(file.name);
                    }}
                    break;
                }}
            }}
            if (uploadFailed) {{
                const reason = uploadError === 'Failed to fetch' || uploadError.toLowerCase().includes('network')
                    ? '文件传输服务已断开，请在电脑端重新打开文件传输并扫码。'
                    : `上传失败：${{uploadError || '未知错误'}}`;
                alert(`${{file.name}}\n${{reason}}`);
                isUploading = false;
                return;
            }}
            if (batchRow) {{
                batchRow.querySelector('.batch-file-status').textContent = `${{formatBytes(file.size)}} · 已完成`;
            }} else {{
                progressWrapper.remove();
                el.querySelector('.bubble').innerHTML += ' <span style="color:#4caf50">✓</span>';
            }}
            isUploading = false;
        }}

        // Dragon-drop support
        document.addEventListener('dragover', e => e.preventDefault());
        document.addEventListener('drop', async e => {{
            e.preventDefault();
            const files = Array.from(e.dataTransfer.files);
            if (files.length) {{
                 const batch = files.length > 1 ? {{
                     id: 'mobile_' + Date.now() + '_' + Math.random().toString(36).slice(2, 8),
                     name: files.length + ' 个文件',
                     total: files.length,
                     totalSize: files.reduce((sum, file) => sum + file.size, 0)
                 }} : null;
                 for(let index = 0; index < files.length; index++) {{
                     await uploadFile(files[index], batch ? {{ ...batch, index }} : null);
                 }}
                 
                 // Small delay for UI and then notify PC
                 setTimeout(() => {{
                     const fileCount = files.length;
                     const replyEl = createMessageElement('received', `ACK: <b>${{fileCount}}</b> FILES SAVED.`, 'System', 'pc');
                     chatBox.appendChild(replyEl);
                     scrollToBottom();
                 }}, 800);
            }}
        }});

    </script>
</body>
</html>
    "#,
        theme = theme,
        mode_class = mode_class,
        theme_css = theme_css,
        logo_base64 = logo_base64
    )
}

#[cfg(test)]
mod tests {
    use super::{file_transfer_theme_css, render_index};

    const BUILT_IN_THEMES: &[&str] = &[
        "retro",
        "sticky-note",
        "mica",
        "acrylic",
        "paper",
        "sakura",
        "terminal",
        "receipt",
        "ink",
    ];

    #[test]
    fn every_desktop_theme_has_a_mobile_file_transfer_variant() {
        let themes_source = include_str!("../../../../src/shared/config/themes.ts");
        for theme in BUILT_IN_THEMES {
            assert!(
                themes_source.contains(&format!("id: \"{theme}\"")),
                "desktop theme list no longer contains {theme}"
            );
            let css = file_transfer_theme_css(theme, "light");
            assert!(
                css.contains("--send-button-background:"),
                "file transfer theme {theme} is incomplete"
            );
        }

        let desktop_theme_count = themes_source.matches("    id: \"").count();
        assert_eq!(
            desktop_theme_count,
            BUILT_IN_THEMES.len(),
            "update the mobile file transfer theme mapping when adding a desktop theme"
        );
    }

    #[test]
    fn rendered_page_uses_the_requested_theme_and_valid_extra_css() {
        let html = render_index("sakura", "dark", "");
        assert!(html.contains(r#"body class="theme-sakura dark-mode""#));
        assert!(html.contains("body.theme-sakura {"));
        assert!(!html.contains("body.theme-sakura {{"));
    }

    #[test]
    fn rendered_mobile_script_has_valid_javascript_syntax() {
        let html = render_index("mica", "light", "");
        let script_start = html.rfind("<script>").unwrap() + "<script>".len();
        let script_end = html.rfind("</script>").unwrap();
        let script = &html[script_start..script_end];
        let path = std::env::temp_dir().join(format!(
            "tiez-file-transfer-{}.js",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&path, script).unwrap();
        let result = std::process::Command::new("node")
            .arg("--check")
            .arg(&path)
            .output();
        let _ = std::fs::remove_file(path);
        if let Ok(output) = result {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
