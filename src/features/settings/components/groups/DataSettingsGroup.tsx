import { open, ask, message } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { ArchiveRestore, ChevronDown, ChevronRight } from "lucide-react";
import { withNativeDialog } from "../../../../shared/lib/focus";
import { isMacPlatform, isWindowsPlatform } from "../../../../shared/lib/platform";

interface DataSettingsGroupProps {
    t: (key: string) => string;
    collapsed: boolean;
    onToggle: () => void;
    dataPath: string;
}

interface ImportReport {
    source: string;
    scanned: number;
    imported: number;
    duplicates: number;
    unsupported: number;
    failed: number;
    warnings: string[];
}

interface DiscoveredImportSource {
    source: string;
    path: string;
    modifiedAt: number;
}

const DataSettingsGroup = ({ t, collapsed, onToggle, dataPath }: DataSettingsGroupProps) => {
    const [importing, setImporting] = useState(false);
    const importSource = isMacPlatform()
        ? {
            titleKey: 'third_party_import_maccy',
            hintKey: 'third_party_import_hint_maccy',
            buttonKey: 'third_party_import_button_maccy',
            chooseKey: 'third_party_import_choose_maccy',
            fileTypeKey: 'third_party_import_file_type_maccy',
            extensions: ['sqlite', 'sqlite3']
        }
        : isWindowsPlatform()
            ? {
                titleKey: 'third_party_import_ditto',
                hintKey: 'third_party_import_hint_ditto',
                buttonKey: 'third_party_import_button_ditto',
                chooseKey: 'third_party_import_choose_ditto',
                fileTypeKey: 'third_party_import_file_type_ditto',
                extensions: ['db', 'dto']
            }
            : {
                titleKey: 'third_party_import',
                hintKey: 'third_party_import_hint',
                buttonKey: 'third_party_import_button',
                chooseKey: 'third_party_import_choose',
                fileTypeKey: 'third_party_import_file_type',
                extensions: ['db', 'dto', 'sqlite', 'sqlite3']
            };

    const importThirdPartyData = async () => {
        if (importing) return;
        setImporting(true);
        try {
            await withNativeDialog(async () => {
                const discovered = await invoke<DiscoveredImportSource[]>(
                    'discover_clipboard_import_sources'
                );
                let selectedPath: string | null = null;
                let confirmed = false;

                if (discovered.length > 0) {
                    const detected = discovered[0];
                    confirmed = await ask(
                        t('third_party_import_found')
                            .replace('{source}', detected.source)
                            .replace('{path}', detected.path),
                        {
                            title: t(importSource.titleKey),
                            kind: 'info',
                            okLabel: t('third_party_import_start'),
                            cancelLabel: t('third_party_import_choose_manually')
                        }
                    );
                    if (confirmed) {
                        selectedPath = detected.path;
                    }
                }

                if (!selectedPath) {
                    const selected = await open({
                        directory: false,
                        multiple: false,
                        title: t(importSource.chooseKey),
                        filters: [
                            {
                                name: t(importSource.fileTypeKey),
                                extensions: importSource.extensions
                            }
                        ]
                    });
                    if (!selected || Array.isArray(selected)) return;
                    selectedPath = selected;
                }

                if (!confirmed) {
                    confirmed = await ask(
                        t('third_party_import_confirm'),
                        {
                            title: t(importSource.titleKey),
                            kind: 'info',
                            okLabel: t('third_party_import_start'),
                            cancelLabel: t('cancel')
                        }
                    );
                    if (!confirmed) return;
                }

                const report = await invoke<ImportReport>('import_clipboard_data', {
                    path: selectedPath
                });
                const summary = t('third_party_import_result')
                    .replace('{source}', report.source)
                    .replace('{scanned}', String(report.scanned))
                    .replace('{imported}', String(report.imported))
                    .replace('{duplicates}', String(report.duplicates))
                    .replace('{unsupported}', String(report.unsupported))
                    .replace('{failed}', String(report.failed));
                const warningText = report.warnings.length > 0
                    ? `\n\n${t('third_party_import_warnings')}\n${report.warnings.join('\n')}`
                    : '';
                await message(`${summary}${warningText}`, {
                    title: t('third_party_import_complete'),
                    kind: report.failed > 0 ? 'warning' : 'info'
                });
            }, "settings:import-third-party-data");
        } catch (error: unknown) {
            console.error(error);
            const errorMessage = error instanceof Error ? error.message : String(error);
            await withNativeDialog(() => message(
                t('third_party_import_failed').replace('{e}', errorMessage),
                { title: t('error'), kind: 'error' }
            ), "settings:import-third-party-data-error");
        } finally {
            setImporting(false);
        }
    };

    return (
    <div className={`settings-group ${collapsed ? 'collapsed' : ''}`}>
        <div className="group-header" onClick={onToggle}>
            <h3 style={{ margin: 0 }}>{t('data_management')}</h3>
            {collapsed ? <ChevronRight size={16} /> : <ChevronDown size={16} />}
        </div>
        {!collapsed && (
            <div className="group-content">
                <div className="setting-item column no-border">
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                        <span className="item-label" style={{ textTransform: 'uppercase', fontSize: '11px', opacity: 0.8 }}>{t('data_path')}</span>
                        <div style={{ display: 'flex', gap: '8px' }}>
                            <button
                                className="btn-icon"
                                onClick={async () => {
                                    await withNativeDialog(async () => {
                                        const selected = await open({
                                            directory: true,
                                            multiple: false,
                                            title: t('change_data_path')
                                        });
                                        if (!selected) return;

                                        const newPath = selected as string;
                                        const confirmed = await ask(
                                            t('data_move_confirm').replace('{path}', newPath),
                                            { title: t('change_data_path'), kind: 'warning', okLabel: t('confirm'), cancelLabel: t('cancel') }
                                        );
                                        if (!confirmed) return;

                                        try {
                                            // The backend migrates the database on the next launch,
                                            // after the current connection has been released.
                                            await invoke("set_data_path", { newPath });
                                            await message(
                                                t('data_move_success'),
                                                { title: t('notice'), kind: 'info' }
                                            );
                                            await invoke("relaunch");
                                        } catch (e: unknown) {
                                            console.error(e);
                                            const errorMsg = e instanceof Error ? e.message : String(e);
                                            await message(
                                                t('data_move_failed').replace('{e}', errorMsg),
                                                { title: t('error'), kind: 'error' }
                                            );
                                        }
                                    }, "settings:change-data-path");
                                }}
                                style={{ width: 'auto', padding: '4px 12px', fontSize: '10px', textTransform: 'uppercase', height: '24px' }}
                            >
                                {t('change_app')}
                            </button>
                            <button
                                className="btn-icon"
                                onClick={async () => {
                                    try {
                                        if (!dataPath) {
                                            throw new Error(t('not_set'));
                                        }
                                        await invoke("open_folder", { path: dataPath });
                                    } catch (e: unknown) {
                                        console.error(e);
                                        const errorMsg = e instanceof Error ? e.message : String(e);
                                        await withNativeDialog(() => message(
                                            `${t('open_failed')}${errorMsg}`,
                                            { title: t('error'), kind: 'error' }
                                        ), "settings:data-path-error");
                                    }
                                }}
                                title={t('open_folder') || "Open"}
                                style={{ width: 'auto', padding: '4px 12px', fontSize: '10px', textTransform: 'uppercase', height: '24px' }}
                            >
                                {t('open_folder')}
                            </button>
                        </div>
                    </div>
                    <div className="data-panel" style={{ fontSize: '11px', color: 'var(--text-secondary)', wordBreak: 'break-all' }}>
                        {dataPath}
                    </div>
                </div>
                <div className="setting-item column no-border">
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '16px' }}>
                        <div>
                            <div className="item-label">{t(importSource.titleKey)}</div>
                            <div className="item-description" style={{ marginTop: '4px' }}>
                                {t(importSource.hintKey)}
                            </div>
                        </div>
                        <button
                            type="button"
                            className="btn-icon"
                            onClick={importThirdPartyData}
                            disabled={importing}
                            style={{
                                width: 'auto',
                                minWidth: '112px',
                                padding: '6px 12px',
                                fontSize: '11px',
                                height: '30px',
                                display: 'inline-flex',
                                alignItems: 'center',
                                justifyContent: 'center',
                                gap: '6px'
                            }}
                        >
                            <ArchiveRestore size={14} />
                            {importing ? t('third_party_importing') : t(importSource.buttonKey)}
                        </button>
                    </div>
                </div>
            </div>
        )}
    </div>
    );
};

export default DataSettingsGroup;
