// Tauri API - will be initialized after DOM loads
let invoke, sendNotification, isPermissionGranted, requestPermission;
let t, setLanguage, updateUI_i18n, getCurrentLanguage, initI18n;

// Initialize APIs safely
function initializeAPIs() {
    console.log('Initializing APIs...');

    // Check Tauri API
    if (!window.__TAURI__) {
        console.error('ERROR: window.__TAURI__ is not available!');
        throw new Error('Tauri API not loaded! Please restart the app.');
    }
    console.log('Tauri API found:', window.__TAURI__);

    // Initialize Tauri API
    ({ invoke } = window.__TAURI__.tauri);
    ({ sendNotification, isPermissionGranted, requestPermission } = window.__TAURI__.notification);
    console.log('invoke function:', invoke);

    // Check i18n API
    if (!window.i18n) {
        console.error('ERROR: window.i18n is not available!');
        throw new Error('i18n not loaded!');
    }

    // Initialize i18n API
    ({ t, setLanguage, updateUI: updateUI_i18n, getCurrentLanguage, init: initI18n } = window.i18n);
    console.log('i18n API initialized');
}

// State management
let previousMiners = new Map();
let notificationsEnabled = false;
let isLoading = false;
let errorCount = 0;
const MAX_ERROR_COUNT = 3;
let refreshInterval = 2000; // Default: 2 seconds
let refreshIntervalId = null;

// Metaverse World
let metaverseWorld = null;

// Show error message to user
function showError(message) {
    const errorDiv = document.createElement('div');
    errorDiv.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        background: #f44336;
        color: white;
        padding: 15px 20px;
        border-radius: 8px;
        box-shadow: 0 4px 12px rgba(0,0,0,0.3);
        z-index: 1000;
        animation: slideIn 0.3s ease-out;
    `;
    errorDiv.textContent = `⚠️ ${message}`;
    document.body.appendChild(errorDiv);

    setTimeout(() => {
        errorDiv.style.animation = 'slideOut 0.3s ease-in';
        setTimeout(() => errorDiv.remove(), 300);
    }, 5000);
}

// Initialize notifications
async function initNotifications() {
    try {
        let permissionGranted = await isPermissionGranted();

        // Always request permission on first launch
        if (!permissionGranted) {
            const permission = await requestPermission();
            permissionGranted = permission === 'granted';
        }

        notificationsEnabled = permissionGranted;

        // Update checkbox to reflect current state
        const notificationsToggle = document.getElementById('notifications-toggle');
        if (notificationsToggle) {
            notificationsToggle.checked = permissionGranted;
        }

        // ALWAYS show reminder on first launch (using localStorage to track)
        const hasSeenNotificationReminder = localStorage.getItem('hasSeenNotificationReminder');

        if (!hasSeenNotificationReminder) {
            // First time user - always show welcome notification setup
            setTimeout(() => {
                showNotificationWelcome();
            }, 1500);
        } else if (!permissionGranted) {
            // Returning user without permission - show reminder
            console.log('Notifications not permitted. Enable in System Preferences to get alerts.');
            setTimeout(() => {
                showNotificationPermissionReminder();
            }, 2000);
        } else {
            console.log('✓ Notifications enabled');
        }
    } catch (error) {
        console.error('Failed to initialize notifications:', error);
        notificationsEnabled = false;
    }
}

// Show welcome notification setup (first time only)
function showNotificationWelcome() {
    const welcomeDiv = document.createElement('div');
    welcomeDiv.style.cssText = `
        position: fixed;
        bottom: 20px;
        right: 20px;
        background: rgba(30, 30, 30, 0.98);
        backdrop-filter: blur(20px);
        color: white;
        padding: 24px 28px;
        border-radius: 16px;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.6);
        z-index: 1000;
        max-width: 420px;
        border: 1px solid rgba(255, 255, 255, 0.15);
        animation: slideInRight 0.4s cubic-bezier(0.16, 1, 0.3, 1);
    `;

    welcomeDiv.innerHTML = `
        <div style="display: flex; align-items: start; gap: 16px;">
            <div style="font-size: 48px; flex-shrink: 0;">👋</div>
            <div style="flex: 1;">
                <div style="font-weight: 700; margin-bottom: 10px; font-size: 1.2rem; color: #fff;">
                    Welcome to ClaudeMiner!
                </div>
                <div style="color: #ddd; font-size: 0.95rem; line-height: 1.6; margin-bottom: 18px;">
                    Get notified instantly when your Claude Code sessions complete tasks or become idle
                </div>
                <div style="display: flex; gap: 10px;">
                    <button onclick="this.parentElement.parentElement.parentElement.parentElement.remove(); localStorage.setItem('hasSeenNotificationReminder', 'true');"
                            style="flex: 1; padding: 12px 18px; background: rgba(255,255,255,0.08); border: 1px solid rgba(255,255,255,0.2); color: white; border-radius: 10px; cursor: pointer; font-size: 0.95rem; font-weight: 500; transition: all 0.2s;">
                        Skip
                    </button>
                    <button onclick="enableNotificationsNow(); this.parentElement.parentElement.parentElement.parentElement.remove();"
                            style="flex: 2; padding: 12px 18px; background: linear-gradient(135deg, #4CAF50, #45a049); border: none; color: white; border-radius: 10px; cursor: pointer; font-weight: 700; font-size: 0.95rem; box-shadow: 0 4px 12px rgba(76, 175, 80, 0.3); transition: all 0.2s;">
                        Enable Notifications 🔔
                    </button>
                </div>
            </div>
        </div>
    `;

    document.body.appendChild(welcomeDiv);

    // Mark as seen after 15 seconds even if no action
    setTimeout(() => {
        if (welcomeDiv.parentElement) {
            localStorage.setItem('hasSeenNotificationReminder', 'true');
            welcomeDiv.style.animation = 'slideOutRight 0.3s ease-in';
            setTimeout(() => welcomeDiv.remove(), 300);
        }
    }, 15000);
}

// Enable notifications from welcome screen
async function enableNotificationsNow() {
    localStorage.setItem('hasSeenNotificationReminder', 'true');

    try {
        const permission = await requestPermission();
        const granted = permission === 'granted';

        notificationsEnabled = granted;

        const notificationsToggle = document.getElementById('notifications-toggle');
        if (notificationsToggle) {
            notificationsToggle.checked = granted;
        }

        if (granted) {
            // Show success message
            const successDiv = document.createElement('div');
            successDiv.style.cssText = `
                position: fixed;
                bottom: 20px;
                right: 20px;
                background: rgba(76, 175, 80, 0.95);
                backdrop-filter: blur(20px);
                color: white;
                padding: 16px 24px;
                border-radius: 12px;
                box-shadow: 0 10px 40px rgba(76, 175, 80, 0.4);
                z-index: 1000;
                font-weight: 600;
                animation: slideInRight 0.3s ease-out;
            `;
            successDiv.textContent = '✓ Notifications enabled successfully!';
            document.body.appendChild(successDiv);

            setTimeout(() => {
                successDiv.style.animation = 'slideOutRight 0.3s ease-in';
                setTimeout(() => successDiv.remove(), 300);
            }, 3000);
        } else {
            // Show settings instruction
            const instructionDiv = document.createElement('div');
            instructionDiv.style.cssText = `
                position: fixed;
                bottom: 20px;
                right: 20px;
                background: rgba(255, 152, 0, 0.95);
                backdrop-filter: blur(20px);
                color: white;
                padding: 16px 24px;
                border-radius: 12px;
                box-shadow: 0 10px 40px rgba(255, 152, 0, 0.4);
                z-index: 1000;
                max-width: 360px;
                line-height: 1.5;
                animation: slideInRight 0.3s ease-out;
            `;
            instructionDiv.innerHTML = `
                <strong>⚙️ Enable in System Settings:</strong><br>
                System Settings → Notifications → ClaudeMiner
            `;
            document.body.appendChild(instructionDiv);

            setTimeout(() => {
                instructionDiv.style.animation = 'slideOutRight 0.3s ease-in';
                setTimeout(() => instructionDiv.remove(), 300);
            }, 6000);
        }
    } catch (error) {
        console.error('Failed to enable notifications:', error);
    }
}

// Make function globally available
window.enableNotificationsNow = enableNotificationsNow;

// Show notification permission reminder
function showNotificationPermissionReminder() {
    const reminderDiv = document.createElement('div');
    reminderDiv.style.cssText = `
        position: fixed;
        bottom: 20px;
        right: 20px;
        background: rgba(30, 30, 30, 0.98);
        backdrop-filter: blur(20px);
        color: white;
        padding: 20px 24px;
        border-radius: 16px;
        box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
        z-index: 1000;
        max-width: 360px;
        border: 1px solid rgba(255, 255, 255, 0.1);
        animation: slideInRight 0.4s cubic-bezier(0.16, 1, 0.3, 1);
    `;

    reminderDiv.innerHTML = `
        <div style="display: flex; align-items: start; gap: 16px;">
            <div style="font-size: 32px; flex-shrink: 0;">🔔</div>
            <div style="flex: 1;">
                <div style="font-weight: 600; margin-bottom: 8px; font-size: 1.05rem;">
                    Enable Notifications
                </div>
                <div style="color: #ccc; font-size: 0.9rem; line-height: 1.5; margin-bottom: 16px;">
                    Get notified when your Claude sessions complete tasks
                </div>
                <div style="display: flex; gap: 8px;">
                    <button onclick="this.parentElement.parentElement.parentElement.parentElement.remove()"
                            style="flex: 1; padding: 10px 16px; background: rgba(255,255,255,0.1); border: 1px solid rgba(255,255,255,0.2); color: white; border-radius: 8px; cursor: pointer; font-size: 0.9rem; transition: all 0.2s;">
                        Maybe Later
                    </button>
                    <button onclick="document.getElementById('settings-btn').click(); this.parentElement.parentElement.parentElement.parentElement.remove();"
                            style="flex: 1; padding: 10px 16px; background: #4CAF50; border: none; color: white; border-radius: 8px; cursor: pointer; font-weight: 600; font-size: 0.9rem; transition: all 0.2s;">
                        Enable
                    </button>
                </div>
            </div>
        </div>
    `;

    document.body.appendChild(reminderDiv);

    // Auto-dismiss after 10 seconds
    setTimeout(() => {
        if (reminderDiv.parentElement) {
            reminderDiv.style.animation = 'slideOutRight 0.3s ease-in';
            setTimeout(() => reminderDiv.remove(), 300);
        }
    }, 10000);
}

// Add CSS animations for reminder
const style = document.createElement('style');
style.textContent = `
    @keyframes slideInRight {
        from {
            transform: translateX(100%);
            opacity: 0;
        }
        to {
            transform: translateX(0);
            opacity: 1;
        }
    }

    @keyframes slideOutRight {
        from {
            transform: translateX(0);
            opacity: 1;
        }
        to {
            transform: translateX(100%);
            opacity: 0;
        }
    }
`;
document.head.appendChild(style);

// Get icon for miner status
function getMinerIcon(status) {
    switch (status) {
        case 'working':
            return '⛏️';
        case 'resting':
            return '😴';
        case 'zombie':
            return '👻';
        default:
            return '👷';
    }
}

// Create miner card element
function createMinerCard(miner) {
    const card = document.createElement('div');
    card.className = `miner-card ${miner.status}`;
    card.dataset.pid = miner.pid;

    const badge = document.createElement('div');
    badge.className = 'miner-badge';
    badge.textContent = `#${miner.pid}`;

    const icon = document.createElement('div');
    icon.className = 'miner-icon';
    icon.textContent = getMinerIcon(miner.status);

    const info = document.createElement('div');
    info.className = 'miner-info';

    const cpu = document.createElement('span');
    cpu.className = 'miner-cpu';
    cpu.textContent = `${t('cpu')}: ${miner.cpu_usage.toFixed(1)}%`;

    const memory = document.createElement('span');
    memory.textContent = `${t('memory')}: ${(miner.memory / 1024 / 1024).toFixed(0)} MB`;

    info.appendChild(cpu);
    info.appendChild(document.createElement('br'));
    info.appendChild(memory);

    card.appendChild(badge);
    card.appendChild(icon);
    card.appendChild(info);

    // Add kill button for zombie processes
    if (miner.status === 'zombie' || !miner.has_terminal) {
        const killBtn = document.createElement('button');
        killBtn.className = 'kill-button';
        killBtn.textContent = t('killProcess');
        killBtn.onclick = async (e) => {
            e.stopPropagation();
            if (confirm(`${t('confirmKill')}${miner.pid}?`)) {
                try {
                    await invoke('kill_miner', { pid: miner.pid });
                    if (notificationsEnabled) {
                        sendNotification({
                            title: t('title'),
                            body: t('processTerminated', { pid: miner.pid })
                        });
                    }
                    await updateMiners();
                } catch (error) {
                    console.error('Failed to kill process:', error);
                    alert(`${t('errorKillingProcess')}: ${error}`);
                }
            }
        };
        card.appendChild(killBtn);
    }

    // Copy PID on click
    card.addEventListener('click', () => {
        navigator.clipboard.writeText(miner.pid.toString());

        // Visual feedback
        const originalText = badge.textContent;
        badge.textContent = t('processCopied');
        setTimeout(() => {
            badge.textContent = originalText;
        }, 1000);
    });

    return card;
}

// Update the UI with current miners
async function updateMiners() {
    console.log('updateMiners() called');
    if (isLoading) {
        console.log('Already loading, skipping...');
        return;
    }

    isLoading = true;

    try {
        console.log('Calling invoke("get_miners")...');
        const miners = await invoke('get_miners');
        console.log('Received miners:', miners);
        console.log('Number of miners:', miners.length);
        errorCount = 0; // Reset error count on success

        // Counters
        let workingCount = 0;
        let restingCount = 0;
        let zombieCount = 0;

        // Update metaverse world with miners
        if (metaverseWorld) {
            metaverseWorld.updateMiners(miners, handleMinerClick);
        }

        // Process miners for notifications and counting
        miners.forEach(miner => {
            // Check for state changes (for notifications)
            const prevMiner = previousMiners.get(miner.pid);

            if (prevMiner && notificationsEnabled) {
                // Working -> Resting (task completed!)
                if (prevMiner.status === 'working' && miner.status === 'resting') {
                    sendNotification({
                        title: t('taskCompleted'),
                        body: t('taskCompletedBody', { pid: miner.pid })
                    });
                }

                // Normal -> Zombie (terminal closed)
                if (prevMiner.has_terminal && !miner.has_terminal) {
                    sendNotification({
                        title: t('zombieDetected'),
                        body: t('zombieDetectedBody', { pid: miner.pid })
                    });
                }
            }

            // New miner detected
            if (!prevMiner && notificationsEnabled && miners.length > previousMiners.size) {
                sendNotification({
                    title: t('newMiner'),
                    body: t('newMinerBody', { pid: miner.pid })
                });
            }

            // Count by type
            if (!miner.has_terminal) {
                zombieCount++;
            } else if (miner.status === 'working') {
                workingCount++;
            } else {
                restingCount++;
            }

            // Update previous state
            previousMiners.set(miner.pid, miner);
        });

        // Remove deleted miners from previous state
        const currentPids = new Set(miners.map(m => m.pid));
        for (const pid of previousMiners.keys()) {
            if (!currentPids.has(pid)) {
                previousMiners.delete(pid);
            }
        }

        // Update stats
        document.getElementById('total-count').textContent = miners.length;
        document.getElementById('working-count').textContent = workingCount;
        document.getElementById('resting-count').textContent = restingCount;
        document.getElementById('zombie-count').textContent = zombieCount;

        // Update last refresh time
        const now = new Date();
        document.getElementById('last-update').textContent =
            `${t('lastUpdated')}: ${now.toLocaleTimeString()}`;

        // Update system tray with all counts
        try {
            await invoke('update_tray_menu', {
                total: miners.length,
                working: workingCount,
                resting: restingCount,
                zombie: zombieCount
            });
        } catch (error) {
            console.error('Failed to update tray menu:', error);
        }

    } catch (error) {
        console.error('Failed to update miners:', error);
        errorCount++;

        if (errorCount >= MAX_ERROR_COUNT) {
            showError(t('errorFetchingProcesses'));
        }
    } finally {
        isLoading = false;
    }
}

// Setup settings modal
function setupSettings() {
    const settingsBtn = document.getElementById('settings-btn');
    const modal = document.getElementById('settings-modal');
    const closeBtn = document.getElementById('close-modal');
    const notificationsToggle = document.getElementById('notifications-toggle');
    const refreshIntervalSelect = document.getElementById('refresh-interval');
    const languageSelect = document.getElementById('language-select');

    // Open modal
    settingsBtn.addEventListener('click', () => {
        modal.style.display = 'flex';
        // Load current settings
        notificationsToggle.checked = notificationsEnabled;
        refreshIntervalSelect.value = refreshInterval.toString();
        languageSelect.value = getCurrentLanguage();
    });

    // Close modal
    closeBtn.addEventListener('click', () => {
        modal.style.display = 'none';
    });

    // Close on background click
    modal.addEventListener('click', (e) => {
        if (e.target === modal) {
            modal.style.display = 'none';
        }
    });

    // Handle notifications toggle
    notificationsToggle.addEventListener('change', async (e) => {
        if (e.target.checked) {
            await initNotifications();
        } else {
            notificationsEnabled = false;
        }
        console.log('Notifications:', notificationsEnabled ? 'enabled' : 'disabled');
    });

    // Handle refresh interval change
    refreshIntervalSelect.addEventListener('change', (e) => {
        refreshInterval = parseInt(e.target.value);
        restartRefreshInterval();
        console.log('Refresh interval changed to:', refreshInterval, 'ms');
    });

    // Handle language change
    languageSelect.addEventListener('change', (e) => {
        setLanguage(e.target.value);
        console.log('Language changed to:', e.target.value);
    });
}

// Handle miner click in metaverse world
async function handleMinerClick(minerEntity) {
    const pid = minerEntity.pid;
    const miner = await invoke('get_miners').then(miners =>
        miners.find(m => m.pid === pid)
    );

    if (!miner) return;

    // Check if it's a zombie
    if (!miner.has_terminal) {
        const confirmMsg = `${t('confirmKill')}${pid}?`;
        if (confirm(confirmMsg)) {
            try {
                await invoke('kill_miner', { pid });
                if (notificationsEnabled) {
                    sendNotification({
                        title: t('title'),
                        body: t('processTerminated', { pid })
                    });
                }

                await updateMiners();
            } catch (error) {
                console.error('Failed to kill process:', error);
                alert(`${t('errorKillingProcess')}: ${error}`);
            }
        }
    } else {
        // Copy PID to clipboard
        navigator.clipboard.writeText(pid.toString());

        // Show toast notification
        const toast = document.createElement('div');
        toast.style.cssText = `
            position: fixed;
            bottom: 20px;
            right: 20px;
            background: rgba(76, 175, 80, 0.95);
            color: white;
            padding: 12px 20px;
            border-radius: 8px;
            font-size: 0.9rem;
            z-index: 10000;
            animation: slideIn 0.3s ease-out;
        `;
        toast.textContent = `✓ PID ${pid} copied to clipboard`;
        document.body.appendChild(toast);

        setTimeout(() => {
            toast.style.animation = 'slideOut 0.3s ease-in';
            setTimeout(() => toast.remove(), 300);
        }, 2000);
    }
}

// Restart refresh interval
function restartRefreshInterval() {
    if (refreshIntervalId) {
        clearInterval(refreshIntervalId);
    }
    refreshIntervalId = setInterval(updateMiners, refreshInterval);

    // Update refresh rate display
    document.getElementById('refresh-rate').textContent = `${refreshInterval / 1000}s`;
}

// Initialize app
async function init() {
    console.log('=== App Initialization Started ===');

    // Initialize APIs first
    try {
        initializeAPIs();
    } catch (error) {
        console.error('Failed to initialize APIs:', error);
        alert('Failed to initialize app: ' + error.message);
        return;
    }

    // Initialize i18n
    initI18n();
    updateUI_i18n();

    // Initialize metaverse world
    const miningWorldContainer = document.getElementById('mining-world');
    if (miningWorldContainer && window.MetaverseWorld) {
        metaverseWorld = new window.MetaverseWorld(miningWorldContainer);
        console.log('Metaverse World initialized');
    }

    await initNotifications();
    await updateMiners();

    // Setup settings UI
    setupSettings();

    // Auto-refresh
    restartRefreshInterval();

    console.log('ClaudeMiner initialized');
}

// Start the app when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
