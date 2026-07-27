import { QRCodeCanvas } from "qrcode.react";

interface ConfigShareQrProps {
  value: string;
  title: string;
  hint: string;
  emptyHint: string;
  ready: boolean;
}

const ConfigShareQr = ({ value, title, hint, emptyHint, ready }: ConfigShareQrProps) => (
  <div className="config-share-panel">
    <div className="qr-container">
      {ready ? (
        <QRCodeCanvas value={value} size={72} level="M" includeMargin={false} />
      ) : (
        <div className="config-share-qr-placeholder" aria-hidden="true" />
      )}
      <div className="qr-label">SCAN ME</div>
    </div>
    <div className="config-share-copy">
      <div className="scan-title">{title}</div>
      <div className="config-share-hint">{ready ? hint : emptyHint}</div>
    </div>
  </div>
);

export default ConfigShareQr;
