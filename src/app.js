// Tauri API - will be initialized after DOM loads
let invoke, sendNotification, isPermissionGranted, requestPermission, listen;
let t, setLanguage, updateUI_i18n, getCurrentLanguage, initI18n;

// Initialize APIs safely
function initializeAPIs() {
    // Check Tauri API
    if (!window.__TAURI__) {
        console.error('ERROR: window.__TAURI__ is not available!');
        throw new Error('Tauri API not loaded! Please restart the app.');
    }

    // Initialize Tauri API
    ({ invoke } = window.__TAURI__.tauri);
    ({ sendNotification, isPermissionGranted, requestPermission } = window.__TAURI__.notification);
    ({ listen } = window.__TAURI__.event);

    // Check i18n API
    if (!window.i18n) {
        console.error('ERROR: window.i18n is not available!');
        throw new Error('i18n not loaded!');
    }

    // Initialize i18n API
    ({ t, setLanguage, updateUI: updateUI_i18n, getCurrentLanguage, init: initI18n } = window.i18n);
}

// State management
let previousMiners = new Map();
let notificationsEnabled = localStorage.getItem('notificationsEnabled') === 'true';
let isLoading = false;
let errorCount = 0;
const MAX_ERROR_COUNT = 3;

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
        localStorage.setItem('notificationsEnabled', permissionGranted.toString());

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
            setTimeout(() => {
                showNotificationPermissionReminder();
            }, 2000);
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

// Custom confirm dialog (web-style)
function showConfirmDialog(message) {
    return new Promise((resolve) => {
        const overlay = document.createElement('div');
        overlay.style.cssText = `
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: rgba(0, 0, 0, 0.6);
            backdrop-filter: blur(8px);
            display: flex;
            align-items: center;
            justify-content: center;
            z-index: 10000;
            animation: fadeIn 0.2s ease-out;
        `;

        const dialog = document.createElement('div');
        dialog.style.cssText = `
            background: rgba(30, 30, 30, 0.98);
            border-radius: 20px;
            padding: 32px;
            max-width: 420px;
            width: 90%;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.8);
            border: 1px solid rgba(255, 255, 255, 0.1);
            animation: scaleIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
        `;

        dialog.innerHTML = `
            <div style="margin-bottom: 28px;">
                <div style="font-size: 1.3rem; font-weight: 700; color: #fff; margin-bottom: 12px;">
                    프로세스 종료 확인
                </div>
                <div style="color: #ccc; font-size: 1rem; line-height: 1.6;">
                    ${message}
                </div>
            </div>
            <div style="display: flex; gap: 12px; justify-content: flex-end;">
                <button id="cancel-btn" style="
                    padding: 14px 28px;
                    background: rgba(255, 255, 255, 0.08);
                    border: 1px solid rgba(255, 255, 255, 0.2);
                    color: white;
                    border-radius: 12px;
                    cursor: pointer;
                    font-size: 1rem;
                    font-weight: 600;
                    transition: all 0.2s;
                ">취소</button>
                <button id="confirm-btn" style="
                    padding: 14px 28px;
                    background: linear-gradient(135deg, #f44336, #d32f2f);
                    border: none;
                    color: white;
                    border-radius: 12px;
                    cursor: pointer;
                    font-size: 1rem;
                    font-weight: 700;
                    box-shadow: 0 4px 16px rgba(244, 67, 54, 0.4);
                    transition: all 0.2s;
                ">종료</button>
            </div>
        `;

        overlay.appendChild(dialog);
        document.body.appendChild(overlay);

        const cancelBtn = dialog.querySelector('#cancel-btn');
        const confirmBtn = dialog.querySelector('#confirm-btn');

        const close = (result) => {
            overlay.style.animation = 'fadeOut 0.2s ease-in';
            dialog.style.animation = 'scaleOut 0.2s ease-in';
            setTimeout(() => overlay.remove(), 200);
            resolve(result);
        };

        cancelBtn.onclick = () => close(false);
        confirmBtn.onclick = () => close(true);
        overlay.onclick = (e) => {
            if (e.target === overlay) close(false);
        };

        // Hover effects
        cancelBtn.onmouseenter = () => {
            cancelBtn.style.background = 'rgba(255, 255, 255, 0.15)';
        };
        cancelBtn.onmouseleave = () => {
            cancelBtn.style.background = 'rgba(255, 255, 255, 0.08)';
        };
        confirmBtn.onmouseenter = () => {
            confirmBtn.style.transform = 'translateY(-2px)';
            confirmBtn.style.boxShadow = '0 6px 20px rgba(244, 67, 54, 0.5)';
        };
        confirmBtn.onmouseleave = () => {
            confirmBtn.style.transform = 'translateY(0)';
            confirmBtn.style.boxShadow = '0 4px 16px rgba(244, 67, 54, 0.4)';
        };
    });
}

// Show success toast
function showSuccessToast(message) {
    const toast = document.createElement('div');
    toast.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        background: linear-gradient(135deg, #4CAF50, #45a049);
        color: white;
        padding: 16px 24px;
        border-radius: 12px;
        font-size: 1rem;
        font-weight: 600;
        box-shadow: 0 8px 24px rgba(76, 175, 80, 0.4);
        z-index: 10000;
        animation: slideInRight 0.3s ease-out;
        display: flex;
        align-items: center;
        gap: 12px;
    `;
    toast.innerHTML = `
        <div style="font-size: 24px;">✓</div>
        <div>${message}</div>
    `;
    document.body.appendChild(toast);

    setTimeout(() => {
        toast.style.animation = 'slideOutRight 0.3s ease-in';
        setTimeout(() => toast.remove(), 300);
    }, 3000);
}

// Add CSS animations for reminder and dialogs
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

    @keyframes fadeIn {
        from { opacity: 0; }
        to { opacity: 1; }
    }

    @keyframes fadeOut {
        from { opacity: 1; }
        to { opacity: 0; }
    }

    @keyframes scaleIn {
        from {
            transform: scale(0.9);
            opacity: 0;
        }
        to {
            transform: scale(1);
            opacity: 1;
        }
    }

    @keyframes scaleOut {
        from {
            transform: scale(1);
            opacity: 1;
        }
        to {
            transform: scale(0.95);
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
            return '🧟';
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
            const confirmMsg = t('confirmKill', { pid: miner.pid });
            console.log('[Kill Button] Clicked, showing confirm:', confirmMsg);

            const confirmed = await showConfirmDialog(confirmMsg);

            if (confirmed) {
                console.log('[Kill Button] Confirmed, killing PID:', miner.pid);
                try {
                    await invoke('kill_miner', { pid: miner.pid });
                    console.log('[Kill Button] Successfully killed PID:', miner.pid);

                    // Show success message
                    showSuccessToast(`프로세스 #${miner.pid}이(가) 종료되었습니다`);

                    // Update UI immediately (multiple times to ensure cleanup happens)
                    await updateMiners();

                    // Wait for SessionCleaner to process (runs every 60s)
                    setTimeout(async () => {
                        console.log('[Kill Button] Refreshing UI after cleanup delay');
                        await updateMiners();
                    }, 1000);
                } catch (error) {
                    console.error('Failed to kill process:', error);
                    alert(`${t('errorKillingProcess')}: ${error}`);
                }
            } else {
                console.log('[Kill Button] Cancelled');
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
    if (isLoading) {
        return;
    }

    isLoading = true;

    try {
        const miners = await invoke('get_miners');
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

            if (prevMiner) {
                // Working -> Resting (task completed!)
                if (prevMiner.status === 'working' && miner.status === 'resting') {
                    console.log(`🎯 Hook Event Detected: PID ${miner.pid} → resting (Stop event)`);
                    if (notificationsEnabled) {
                        sendNotification({
                            title: t('taskCompleted'),
                            body: t('taskCompletedBody', { pid: miner.pid })
                        });
                    }
                }

                // Resting -> Working (task started!)
                if (prevMiner.status === 'resting' && miner.status === 'working') {
                    console.log(`🎯 Hook Event Detected: PID ${miner.pid} → working (UserPromptSubmit event)`);
                }

                // Normal -> Zombie (terminal closed)
                if (prevMiner.has_terminal && !miner.has_terminal) {
                    console.log(`⚠️ State Change: PID ${miner.pid} → zombie (terminal closed)`);
                    if (notificationsEnabled) {
                        sendNotification({
                            title: t('zombieDetected'),
                            body: t('zombieDetectedBody', { pid: miner.pid })
                        });
                    }
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
    const testNotificationBtn = document.getElementById('test-notification-btn');
    const modal = document.getElementById('settings-modal');
    const closeBtn = document.getElementById('close-modal');
    const notificationsToggle = document.getElementById('notifications-toggle');
    const languageSelect = document.getElementById('language-select');

    // Test notification button
    testNotificationBtn.addEventListener('click', async () => {
        console.log('[TestNotification] Button clicked');
        try {
            const result = await invoke('send_test_notification');
            console.log('[TestNotification] Result:', result);

            // Show visual feedback
            testNotificationBtn.style.transform = 'scale(1.2)';
            setTimeout(() => {
                testNotificationBtn.style.transform = 'scale(1)';
            }, 200);
        } catch (error) {
            console.error('[TestNotification] Failed:', error);
        }
    });

    // Open modal
    settingsBtn.addEventListener('click', () => {
        modal.style.display = 'flex';
        // Load current settings
        notificationsToggle.checked = notificationsEnabled;
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
        localStorage.setItem('notificationsEnabled', notificationsEnabled.toString());
    });

    // Handle language change
    languageSelect.addEventListener('change', (e) => {
        setLanguage(e.target.value);
    });
}

// Handle miner click in metaverse world
async function handleMinerClick(minerEntity) {
    const pid = minerEntity.pid;
    const miner = await invoke('get_miners').then(miners =>
        miners.find(m => m.pid === pid)
    );

    if (!miner) return;

    // Check if it's a zombie (either no terminal or zombie status)
    if (!miner.has_terminal || miner.status === 'zombie') {
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

// Auto-refresh interval
let refreshInterval = null;
const REFRESH_INTERVAL_MS = 3000; // 3 seconds

function restartRefreshInterval() {
    // Clear existing interval if any
    if (refreshInterval) {
        clearInterval(refreshInterval);
    }

    // Set up new interval to refresh every 3 seconds
    refreshInterval = setInterval(() => {
        updateMiners();
    }, REFRESH_INTERVAL_MS);

    console.log(`✅ Auto-refresh started (${REFRESH_INTERVAL_MS / 1000} second interval)`);
}

// Setup Tauri event listeners for real-time updates
async function setupTauriEventListeners() {
    console.log('🎧 Setting up Tauri event listeners...');

    // Listen for session-created events
    await listen('session-created', (event) => {
        console.log('🌟 New session created:', event.payload);
        // Immediately update UI
        updateMiners();
    });

    // Listen for session-status-changed events
    await listen('session-status-changed', (event) => {
        console.log('🔄 Session status changed:', event.payload);
        // Immediately update UI
        updateMiners();
    });

    // Listen for session-terminated events
    await listen('session-terminated', (event) => {
        console.log('💀 Session terminated:', event.payload);
        // Immediately update UI
        updateMiners();
    });

    console.log('✅ Tauri event listeners setup complete');
}

// Initialize app
async function init() {
    console.log('🚀 ClaudeMiner Starting...');

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
    }

    await initNotifications();

    // Setup Tauri event listeners for real-time updates
    await setupTauriEventListeners();

    await updateMiners();

    // Setup settings UI
    setupSettings();

    // Auto-refresh (backup polling for stability)
    restartRefreshInterval();

    console.log('✅ ClaudeMiner Ready');
}

// Start the app when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
