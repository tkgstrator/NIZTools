# Atom66 Windows通信DLLの解析資料

対象はSoftware ZIP内の `66EC(XRGB)BleHWI.dll`。静的解析から得た通信形式と関数アドレスは[詳細レポート](evidence/dll/report.md)に記録している。

レポートに挙げた逆アセンブル、PE import／export、通信処理の抜粋、解析用Pythonは `evidence/dll/` に保存してある。元DLLはZIPのまま保持し、これらの解析資料にはバイナリを同梱しない。

DLLレポートの「hardware descriptor未確認」はDLL単独の調査範囲を示す。親エージェントが後からこのMacのdescriptorを取得した結果は[統合解析メモ](atom66-findings.md)を参照する。版情報応答や設定captureは、両方の調査でまだ取得していない。
