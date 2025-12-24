# 部分的なブラーとゴースト現象の修正計画（Phase 1 安定性への回帰）

## 概要
Phase 1 で動作していた「型抜き（ホール）」が、Phase 2 の GPU ブラー移行後に機能しなくなった問題を修正します。WebView の不透明な要素が GPU の透明度を妨げている可能性を排除し、確実な座標同期を再構築します。

## 修正が必要な点

### 1. ウィンドウ認識ロジックの Phase 1 完全準拠 ([window_manager.rs](file:///c:/Users/atuya/Documents/develop/hamaguri-blur/src-tauri/src/window_manager.rs))
- **過剰なフィルタリングの廃止**: Phase 1 で動作していた単純なループに戻し、システムウィンドウの除外等は Phase 1 と同一の基準にします。
- **境界線補正の統一**: Windows の不可視境界線補正 (-7px) は、Phase 1 と同様に Rust 側で一度だけ適用し、JS/GPU 両方に伝播させます。

### 2. GPU 型抜き（ホール）の確実な動作保証 ([renderer.rs](file:///c:/Users/atuya/Documents/develop/hamaguri-blur/src-tauri/src/renderer.rs) / [blur.wgsl](file:///c:/Users/atuya/Documents/develop/hamaguri-blur/src-tauri/src/blur.wgsl))
- **アルファブレンド設定の見直し**: `ALPHA_BLENDING` ではなく `REPLACE` に近い挙動（穴の部分は確実に 0,0,0,0 を出力）を検討し、WebView の透明度との整合性を取ります。
- **WebView マスクの廃止 ([main.js](file:///c:/Users/atuya/Documents/develop/hamaguri-blur/src/main.js))**: Phase 1 方式の 4 つの div マスクが GPU の「穴」を覆い隠している可能性があるため、これを削除し、GPU の穴あき処理に一本化します。

### 3. 座標変換の整合性確保 ([lib.rs](file:///c:/Users/atuya/Documents/develop/hamaguri-blur/src-tauri/src/lib.rs))
- **物理・論理座標の棲み分け**: Rust 内では物理ピクセルで計算を行い、JS (WebView) に渡す際のみスケール係数で割ります。GPU シェーダーに渡す正規化座標 (0-1) は、monitor 物理サイズで割った値をそのまま使用します。

## 検証方法
- [ ] アクティブウィンドウの部分が「完全に透けて」デスクトップが見えること。
- [ ] 座標がズレることなく、ウィンドウを枠が囲っていること。
- [ ] マルチモニタ環境でモニターを跨いでも追従すること。
