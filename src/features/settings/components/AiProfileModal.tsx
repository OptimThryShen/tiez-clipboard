import AppModal from "../../../shared/components/AppModal";
import type { EditableAiProfile } from "../types";

interface AiProfileModalProps {
    editingProfile: EditableAiProfile | null;
    t: (key: string) => string;
    onClose: () => void;
    onSave: (profile: EditableAiProfile) => void;
    setEditingProfile: (val: EditableAiProfile) => void;
}

const AiProfileModal = ({ editingProfile, t, onClose, onSave, setEditingProfile }: AiProfileModalProps) => (
    <AppModal
        open={Boolean(editingProfile)}
        onClose={onClose}
        panelClassName="modal-panel modal-panel-form"
    >
        {editingProfile && (
            <>
                <h3 className="modal-title">{editingProfile.isNew ? t("ai_add_model") : t("ai_edit_model")}</h3>

                <div>
                    <div className="modal-field-label">Endpoint URL</div>
                    <input
                        className="search-input"
                        style={{ width: "100%" }}
                        value={editingProfile.baseUrl}
                        onChange={(e) => setEditingProfile({ ...editingProfile, baseUrl: e.target.value })}
                    />
                </div>
                <div>
                    <div className="modal-field-label">API Key</div>
                    <input
                        className="search-input"
                        type="password"
                        style={{ width: "100%" }}
                        value={editingProfile.apiKey}
                        onChange={(e) => setEditingProfile({ ...editingProfile, apiKey: e.target.value })}
                    />
                </div>
                <div>
                    <div className="modal-field-label">Model ID</div>
                    <input
                        className="search-input"
                        style={{ width: "100%" }}
                        value={editingProfile.model}
                        onChange={(e) => setEditingProfile({ ...editingProfile, model: e.target.value })}
                        placeholder="gpt-4o"
                    />
                </div>
                <div className="modal-field-row">
                    <span style={{ fontSize: "13px", fontWeight: 600 }}>{t("ai_thinking_enabled")}</span>
                    <label className="switch">
                        <input
                            className="cb"
                            type="checkbox"
                            checked={editingProfile.enableThinking}
                            onChange={(e) => setEditingProfile({ ...editingProfile, enableThinking: e.target.checked })}
                        />
                        <div className="toggle"><div className="left" /><div className="right" /></div>
                    </label>
                </div>
                <div className="modal-panel-actions split">
                    <button type="button" className="modal-button" onClick={onClose}>{t("cancel")}</button>
                    <button type="button" className="modal-button primary" onClick={() => onSave(editingProfile)}>
                        {t("ai_save_profile")}
                    </button>
                </div>
            </>
        )}
    </AppModal>
);

export default AiProfileModal;
