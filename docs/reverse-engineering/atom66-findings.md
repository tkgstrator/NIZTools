# Atom66 EC BLEの解析結果

2026年10月10日、`devonlys-MacBook-Pro.local` 上の `/Users/devonly/Developer/personal/NIZ` を調査した。Atom66 EC-BLEを最初の対象として、Windows版の通信処理をRustでmacOSへ移植する見通しが得られた。設定通信の実機検証はこれから行う。

## 調査したファイル

元フォルダには5個のZIPと.DS_Storeがあり、ソースコードとGitリポジトリはなかった。Software ZIPには次の3ファイルがある。

| ファイル | 展開サイズ | SHA256 |
| --- | ---: | --- |
| 66EC(XRGB)Ble.exe | 2,447,872バイト | 3189ce0a95911b7cbeffd6a15092f38af490aa8bf2d90115453d11f401974555 |
| 66EC(XRGB)BleHWI.dll | 66,048バイト | 1ca6c2141ff2aac1d38c03533ac56a64c27beb9e4e82eae9a1f6c9abf5c108d3 |
| 66EC(XRGB)BleRES.dll | 52,477,440バイト | manifest参照 |

EXEとHWI DLLは32ビットWindows PE形式。macOSでそのDLLを直接利用する方式は採らず、通信形式を実装し直す。RES DLLは今回は解析していない。

他のZIPはEC-BLE、EC-BLE RGB、EC-Sのファームウェア履歴と3種類の旧Word形式マニュアル。全メンバーとハッシュは[archive-manifest.json](evidence/archive-manifest.json)に記録する。Windowsバイナリの実行、ファームウェア書込、設定書込は行っていない。

## macOSで確認した接続機器

IORegistryにNIZ候補のUSB機器があり、設定用collectionは次の情報を持つ。これらは実測値であり、Rustアプリの通信成功を意味しない。

| 項目 | 値 |
| --- | --- |
| USB VID／PID | 0483／502a |
| 製品名／製造者 | 66EC-XRGB／CATEX TECH. |
| 通信 | USB |
| Primary usage page／usage | 008c／1 |
| 最大input／output report | 64／64バイト |
| Feature report | 0バイト |
| Descriptor | 39バイト、Report ID項目なし |

[HID descriptor](evidence/atom66-hid-descriptor.hex)のInputとOutputはいずれもReport Size 64ビット×Report Count 8で、64バイト。USB製品名は汎用的な可能性があるため、機種、RGB有無、ファームウェアはF9の応答を取得してから判定する。

## Windows版で確認した通信

HWI DLLはWindows標準のHID列挙とReadFile／WriteFileを使い、VID 0483、PID 502a／512a／522a、interface 1を選ぶ。DLLの全export、比較処理、通信処理のアドレスは[DLL解析](dll-analysis.md)にまとめた。

内部のpayloadは64バイト。Windows I/O側ではreport ID相当の0を前置して65バイトを渡す。`hidapi.write`も先頭にreport IDを要求するが、受信は番号付きreportがある場合にのみreport IDを含むため、送信と受信を混同しない。[hidapi API](https://docs.rs/hidapi/latest/hidapi/struct.HidDevice.html)

| 操作 | payloadの先頭 | 根拠となるDLLのVA |
| --- | --- | --- |
| 版情報読取 | 00 F9 | 10002460 |
| 全キー設定読取 | 00 F2 | 100014d0 |
| 全キー設定書込開始 | 00 F1 | 10001860 |
| キー定義 | 00 F0 layer key type | serializers |
| キー設定書込終端 | 00とF6×63 | 10001a20以降 |
| 使用回数読取 | 00 E3 | 10001b70 |

キー設定の読取終端は受信payload先頭のF6。照明とカウンターの終端判定位置は異なるため、すべての操作に同じ終端処理を使わない。DLLは3レイヤー×66キーを扱い、キー定義はtype 0〜4。マクロ等の複数reportには11バイトheaderと最大53バイトbodyがある。

Windows版には受信長の確認不足や無期限の同期readがある。移植では実際の転送長を確認し、短いreportや欠落終端をエラーにする。公開コードとDLLで一致していない細部は、生captureまたは追加逆アセンブルで解決してから書込可能にする。

遅延を含むマクロでは実際に差がある。DLLのwriterはn個のキーに対して4nバイトを作り、最後のキーにもdelay markerと値を付ける。公開writerは最後のdelayを省き、4n−3バイトにする。DLLのparserはどちらも解析できる構造だが、今回の機器の保存動作は未確認。両形式のfixtureを残し、実機の元レコード形式を保持する。変更したマクロの書込を有効にするには、実機で往復と出力を確認する。

## 設定ファイルと機能

マニュアルは通常・右Fn・左Fnの3レイヤー、単一キー、最大6キーの組合せ、マクロ、反復、メディア、マウス、機能キー、電源キー、PC上の設定保存、使用回数読取を説明している。マクロのユーザー向けdelay範囲は50〜60,000ms。wire形式の範囲を、そのままUIの有効範囲とみなさない。

`.pro`のXML宣言とPredefinedCombo、PredefinedMacro、PredefinedEmu、CurrentSettings、KEY、Level、Mode、HWCode等がEXEの保存処理で確認できる。[保存処理の文字列とraw offset](evidence/profile-format-strings.txt)に証拠を残した。実際のWindows出力標本はまだなく、完全なimport／export互換性は未検証。

マニュアルではOFFICEモードとカスタムモードで読取対象が変わる。モードと電源再投入後の保存動作は実機で確認する。Bluetoothの3スロットとUSB／Bluetooth切替は説明されているが、Bluetooth経由の設定コマンドは確認できていない。公式資料はソフト利用をUSB接続に限定している。[公式案内](https://support.niz.store/portal/ja/kb/articles/download)

## 公開実装との比較

[NIZ Web](https://github.com/cbingb666/niz-web)のcommit `395a8f8e46ab677ecf30536c3e756ab7e68c2683` には、同じ基本コマンド、usage page 008c／usage 1、3または9グループの保持、.pro importがある。9グループの機器では594レコードを保持し、先頭198レコードのみを編集する設計。このMacの機器がどちらの構造を持つかは未確認。

[Ruby版NIZツール](https://github.com/cho45/niz-tools-ruby)もF9、F2、F1／F0／F6を実装している。これらは通信形式を照合する一次資料として参照し、今回のRustアプリの実機検証実績には数えない。参照した公開ファイルのパス、版、ハッシュは[source-provenance.json](evidence/source-provenance.json)に保存する。

## 誤った書込の影響

キー割り当ての誤りとファームウェア更新の誤りは分けて扱う。通常の設定では意図しない入力やFn喪失が想定されるが、不正な形式に対する実機ファームウェアの防御処理は未確認。任意のデータを書いても故障しないとは保証できない。設定領域の破損、入力不能、途中書込の状態は実機で復元手順を含めて検証する必要がある。

ファームウェア書込には別のexportとcolon-prefixed hex送信処理がある。起動に関わる領域や復旧動作を今回確認できていないため、初版ではそのコードと校正コマンドを実装・公開しない。通常の設定applyへ任意のraw commandを渡すCLIも設けない。

## 実機通信前の状態

実行したのはZIP一覧・一時展開、PE解析、逆アセンブル、DOCのテキスト化、公開ソースの閲覧、macOS機器情報とdescriptorの読取。F9を含む設定用のHID出力reportは送っていない。設定・ファームウェア・校正データは変更していない。

LocalGPTとClaude Code MCPの呼出ツールを現在のツール一覧とプラグイン検索で確認できなかったため、Codexの別エージェントがDLL解析とマニュアル解析を分担した。指定されたMCPによるレビューを実施したとは扱わない。

次は[設計案](../superpowers/specs/2026-10-10-niz-macos-design.md)を確定し、Rust codec、読取専用CLI、実機バックアップの順に実装する。
