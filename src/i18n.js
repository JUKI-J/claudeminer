// Multi-language support for ClaudeMiner
const translations = {
    en: {
        // Header
        title: "ClaudeMiner",
        subtitle: "Visual Process Monitor for Claude Code",

        // Stats
        totalMiners: "Total Miners",
        workingMiners: "Working",
        restingMiners: "Resting",
        zombieMiners: "Zombies",

        // Sections
        workingMinersSection: "Working Miners",
        restingMinersSection: "Resting Miners",
        zombieMinersSection: "Zombie Processes",

        // Empty states
        noWorkingMiners: "No miners working",
        noRestingMiners: "No miners resting",
        noZombieProcesses: "No zombie processes",

        // Miner card
        cpu: "CPU",
        memory: "MEM",
        killProcess: "Kill Process",
        confirmKill: "Kill process #",
        processCopied: "✓ Copied!",

        // Settings
        settings: "Settings",
        enableNotifications: "Enable Notifications",
        refreshInterval: "Refresh Interval",
        second: "second",
        seconds: "seconds",
        about: "About",
        version: "ClaudeMiner v1.0.0",
        support: "☕ Buy Me a Coffee",
        language: "Language",

        // Notifications
        taskCompleted: "🎉 Task Completed!",
        taskCompletedBody: "Miner #{pid} has finished working and is now resting.",
        zombieDetected: "⚠️ Zombie Process Detected",
        zombieDetectedBody: "Miner #{pid} has lost its terminal but is still running.",
        newMiner: "⛏️ New Miner Started",
        newMinerBody: "Miner #{pid} has begun working.",
        processTerminated: "Process #{pid} has been terminated",

        // Errors
        errorFetchingProcesses: "Failed to fetch process information. Please check if Claude Code is running.",
        errorKillingProcess: "Failed to kill process",

        // Last update
        lastUpdated: "Last updated",

        // Legend
        legendTip: "Click miner to see PID • Zombies can be terminated"
    },

    ko: {
        // Header
        title: "ClaudeMiner",
        subtitle: "Claude Code 프로세스 모니터",

        // Stats
        totalMiners: "전체 광부",
        workingMiners: "작업중",
        restingMiners: "휴식중",
        zombieMiners: "좀비",

        // Sections
        workingMinersSection: "작업중인 광부",
        restingMinersSection: "휴식중인 광부",
        zombieMinersSection: "좀비 프로세스",

        // Empty states
        noWorkingMiners: "작업중인 광부가 없습니다",
        noRestingMiners: "휴식중인 광부가 없습니다",
        noZombieProcesses: "좀비 프로세스가 없습니다",

        // Miner card
        cpu: "CPU",
        memory: "메모리",
        killProcess: "프로세스 종료",
        confirmKill: "프로세스 #을(를) 종료하시겠습니까? ",
        processCopied: "✓ 복사됨!",

        // Settings
        settings: "설정",
        enableNotifications: "알림 활성화",
        refreshInterval: "새로고침 간격",
        second: "초",
        seconds: "초",
        about: "정보",
        version: "ClaudeMiner v1.0.0",
        support: "☕ 커피 사주기",
        language: "언어",

        // Notifications
        taskCompleted: "🎉 작업 완료!",
        taskCompletedBody: "광부 #{pid}이(가) 작업을 마치고 휴식중입니다.",
        zombieDetected: "⚠️ 좀비 프로세스 발견",
        zombieDetectedBody: "광부 #{pid}이(가) 터미널을 잃었지만 계속 실행중입니다.",
        newMiner: "⛏️ 새 광부 시작",
        newMinerBody: "광부 #{pid}이(가) 작업을 시작했습니다.",
        processTerminated: "프로세스 #{pid}이(가) 종료되었습니다",

        // Errors
        errorFetchingProcesses: "프로세스 정보를 가져오는데 실패했습니다. Claude Code가 실행중인지 확인하세요.",
        errorKillingProcess: "프로세스 종료 실패",

        // Last update
        lastUpdated: "마지막 업데이트",

        // Legend
        legendTip: "광부를 클릭하여 PID 확인 • 좀비 프로세스는 종료 가능"
    },

    ja: {
        // Header
        title: "ClaudeMiner",
        subtitle: "Claude Code プロセスモニター",

        // Stats
        totalMiners: "総マイナー",
        workingMiners: "作業中",
        restingMiners: "休憩中",
        zombieMiners: "ゾンビ",

        // Sections
        workingMinersSection: "作業中のマイナー",
        restingMinersSection: "休憩中のマイナー",
        zombieMinersSection: "ゾンビプロセス",

        // Empty states
        noWorkingMiners: "作業中のマイナーはいません",
        noRestingMiners: "休憩中のマイナーはいません",
        noZombieProcesses: "ゾンビプロセスはありません",

        // Miner card
        cpu: "CPU",
        memory: "メモリ",
        killProcess: "プロセス終了",
        confirmKill: "プロセス #を終了しますか？",
        processCopied: "✓ コピーしました！",

        // Settings
        settings: "設定",
        enableNotifications: "通知を有効化",
        refreshInterval: "更新間隔",
        second: "秒",
        seconds: "秒",
        about: "情報",
        version: "ClaudeMiner v1.0.0",
        support: "☕ コーヒーをおごる",
        language: "言語",

        // Notifications
        taskCompleted: "🎉 タスク完了！",
        taskCompletedBody: "マイナー #{pid}が作業を終えて休憩中です。",
        zombieDetected: "⚠️ ゾンビプロセス検出",
        zombieDetectedBody: "マイナー #{pid}がターミナルを失いましたが、まだ実行中です。",
        newMiner: "⛏️ 新しいマイナー開始",
        newMinerBody: "マイナー #{pid}が作業を開始しました。",
        processTerminated: "プロセス #{pid}が終了しました",

        // Errors
        errorFetchingProcesses: "プロセス情報の取得に失敗しました。Claude Codeが実行中か確認してください。",
        errorKillingProcess: "プロセスの終了に失敗しました",

        // Last update
        lastUpdated: "最終更新",

        // Legend
        legendTip: "マイナーをクリックするとPIDを確認できます • ゾンビプロセスは終了できます"
    },

    es: {
        // Header
        title: "ClaudeMiner",
        subtitle: "Monitor de Procesos para Claude Code",

        // Stats
        totalMiners: "Total de Mineros",
        workingMiners: "Trabajando",
        restingMiners: "Descansando",
        zombieMiners: "Zombies",

        // Sections
        workingMinersSection: "Mineros Trabajando",
        restingMinersSection: "Mineros Descansando",
        zombieMinersSection: "Procesos Zombie",

        // Empty states
        noWorkingMiners: "No hay mineros trabajando",
        noRestingMiners: "No hay mineros descansando",
        noZombieProcesses: "No hay procesos zombie",

        // Miner card
        cpu: "CPU",
        memory: "MEM",
        killProcess: "Terminar Proceso",
        confirmKill: "¿Terminar proceso #",
        processCopied: "✓ ¡Copiado!",

        // Settings
        settings: "Configuración",
        enableNotifications: "Activar Notificaciones",
        refreshInterval: "Intervalo de Actualización",
        second: "segundo",
        seconds: "segundos",
        about: "Acerca de",
        version: "ClaudeMiner v1.0.0",
        support: "☕ Invítame un Café",
        language: "Idioma",

        // Notifications
        taskCompleted: "🎉 ¡Tarea Completada!",
        taskCompletedBody: "El minero #{pid} ha terminado de trabajar y está descansando.",
        zombieDetected: "⚠️ Proceso Zombie Detectado",
        zombieDetectedBody: "El minero #{pid} ha perdido su terminal pero sigue ejecutándose.",
        newMiner: "⛏️ Nuevo Minero Iniciado",
        newMinerBody: "El minero #{pid} ha comenzado a trabajar.",
        processTerminated: "El proceso #{pid} ha sido terminado",

        // Errors
        errorFetchingProcesses: "Error al obtener información de procesos. Verifica que Claude Code esté ejecutándose.",
        errorKillingProcess: "Error al terminar proceso",

        // Last update
        lastUpdated: "Última actualización",

        // Legend
        legendTip: "Haz clic en el minero para ver PID • Los zombies pueden terminarse"
    }
};

// Current language (default: browser language or English)
let currentLanguage = 'en';

// Initialize language from browser or localStorage
function initLanguage() {
    // Check localStorage first
    const savedLanguage = localStorage.getItem('claudeminer-language');
    if (savedLanguage && translations[savedLanguage]) {
        currentLanguage = savedLanguage;
        return;
    }

    // Detect browser language
    const browserLang = navigator.language.toLowerCase();
    if (browserLang.startsWith('ko')) {
        currentLanguage = 'ko';
    } else if (browserLang.startsWith('ja')) {
        currentLanguage = 'ja';
    } else if (browserLang.startsWith('es')) {
        currentLanguage = 'es';
    } else {
        currentLanguage = 'en';
    }

    localStorage.setItem('claudeminer-language', currentLanguage);
}

// Get translated text
function translate(key, replacements = {}) {
    let text = translations[currentLanguage][key] || translations.en[key] || key;

    // Replace placeholders like {pid}
    Object.keys(replacements).forEach(placeholder => {
        text = text.replace(`{${placeholder}}`, replacements[placeholder]);
    });

    return text;
}

// Change language
function changeLanguage(lang) {
    if (translations[lang]) {
        currentLanguage = lang;
        localStorage.setItem('claudeminer-language', lang);
        updateAllUIText();
    }
}

// Update all UI text to current language
function updateAllUIText() {
    console.log('Updating UI language...');

    try {
        // Header
        const title = document.querySelector('h1');
        if (title) title.textContent = `🪨 ${translate('title')}`;

        const subtitle = document.querySelector('.subtitle');
        if (subtitle) subtitle.textContent = translate('subtitle');

        // Stats labels - actual structure uses .stat not .stat-card
        const stats = document.querySelectorAll('.stat .stat-label');
        if (stats[0]) stats[0].textContent = 'Active Sessions';
        if (stats[1]) stats[1].textContent = '⛏️ Working';
        if (stats[2]) stats[2].textContent = '😴 Resting';
        if (stats[3]) stats[3].textContent = '👻 Zombie';

        // Settings modal - safe access
        const notifLabel = document.querySelectorAll('.setting-item label')[0];
        if (notifLabel && notifLabel.childNodes[2]) {
            notifLabel.childNodes[2].textContent = ` ${translate('enableNotifications')}`;
        }

        const refreshLabel = document.querySelector('label[for="refresh-interval"]');
        if (refreshLabel) refreshLabel.textContent = translate('refreshInterval');

        const langLabel = document.querySelector('label[for="language-select"]');
        if (langLabel) langLabel.textContent = translate('language');

        // Legend tip
        const legendTip = document.querySelector('.legend-tip');
        if (legendTip) legendTip.textContent = translate('legendTip');

        console.log('UI language updated successfully');
    } catch (error) {
        console.error('Error updating UI language:', error);
    }
}

// Export functions
window.i18n = {
    init: initLanguage,
    t: translate,
    setLanguage: changeLanguage,
    updateUI: updateAllUIText,
    getCurrentLanguage: () => currentLanguage,
    getAvailableLanguages: () => Object.keys(translations)
};
