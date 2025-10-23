// Metaverse-style Miner Movement System
// Handles 2D space movement, collision detection, and animations

class MinerEntity {
    constructor(miner, worldWidth, worldHeight) {
        this.pid = miner.pid;
        this.status = miner.status;
        this.has_terminal = miner.has_terminal;
        this.cpu_usage = miner.cpu_usage;
        this.memory = miner.memory;
        this.name = miner.name;

        // Size
        this.size = 50; // 50x50px miner character

        // Position (random initial position)
        this.x = Math.random() * (worldWidth - this.size);
        this.y = Math.random() * (worldHeight - this.size);

        // Velocity
        this.vx = 0;
        this.vy = 0;
        this.speed = this.getSpeedByStatus();

        // Target position for movement
        this.targetX = this.x;
        this.targetY = this.y;

        // Animation frame
        this.frame = 0;
        this.animationSpeed = this.getAnimationSpeedByStatus();

        // DOM element
        this.element = null;

        // World bounds
        this.worldWidth = worldWidth;
        this.worldHeight = worldHeight;

        // Movement timer
        this.moveTimer = 0;
        this.moveInterval = this.getMoveIntervalByStatus();
    }

    getSpeedByStatus() {
        switch (this.status) {
            case 'working':
                return 0.5; // Slow, focused on mining
            case 'resting':
                return 0.3; // Very slow wandering
            case 'zombie':
            default:
                return 1.5; // Fast, erratic movement
        }
    }

    getAnimationSpeedByStatus() {
        switch (this.status) {
            case 'working':
                return 10; // Pickaxe swinging animation
            case 'resting':
                return 30; // Slow breathing animation
            case 'zombie':
            default:
                return 5; // Fast floating animation
        }
    }

    getMoveIntervalByStatus() {
        switch (this.status) {
            case 'working':
                return 3000; // Change direction every 3 seconds
            case 'resting':
                return 5000; // Change direction every 5 seconds
            case 'zombie':
            default:
                return 2000; // Change direction every 2 seconds (erratic)
        }
    }

    getTargetZone() {
        const zoneHeight = this.worldHeight / 3;
        switch (this.status) {
            case 'working':
                return { minY: 0, maxY: zoneHeight }; // Top zone
            case 'resting':
                return { minY: zoneHeight, maxY: zoneHeight * 2 }; // Middle zone
            case 'zombie':
            default:
                return { minY: zoneHeight * 2, maxY: this.worldHeight }; // Bottom zone
        }
    }

    setNewTarget() {
        const zone = this.getTargetZone();
        const margin = this.size;

        // Bias towards target zone but allow some wandering
        const biasStrength = 0.7; // 70% bias towards target zone

        if (Math.random() < biasStrength) {
            // Move towards target zone
            this.targetX = margin + Math.random() * (this.worldWidth - margin * 2);
            this.targetY = zone.minY + Math.random() * (zone.maxY - zone.minY - margin);
        } else {
            // Occasional wandering anywhere
            this.targetX = margin + Math.random() * (this.worldWidth - margin * 2);
            this.targetY = margin + Math.random() * (this.worldHeight - margin * 2);
        }
    }

    update(deltaTime, otherMiners) {
        // Update animation frame
        this.frame = (this.frame + 1) % (this.animationSpeed * 4);

        // Update movement timer
        this.moveTimer += deltaTime;
        if (this.moveTimer >= this.moveInterval) {
            this.setNewTarget();
            this.moveTimer = 0;
        }

        // Calculate direction to target
        const dx = this.targetX - this.x;
        const dy = this.targetY - this.y;
        const distance = Math.sqrt(dx * dx + dy * dy);

        if (distance > 5) {
            // Move towards target
            this.vx = (dx / distance) * this.speed;
            this.vy = (dy / distance) * this.speed;

            // Update position
            this.x += this.vx;
            this.y += this.vy;
        } else {
            // Reached target, set new one
            this.setNewTarget();
        }

        // Keep within bounds
        this.x = Math.max(0, Math.min(this.worldWidth - this.size, this.x));
        this.y = Math.max(0, Math.min(this.worldHeight - this.size, this.y));

        // Collision detection with other miners
        this.handleCollisions(otherMiners);

        // Update DOM position with hardware acceleration
        if (this.element) {
            this.element.style.transform = `translate(${this.x}px, ${this.y}px) translateZ(0)`;

            // Flip sprite based on movement direction (sprite only, not the whole element)
            const sprite = this.element.querySelector('.miner-sprite');
            if (sprite) {
                if (this.vx < -0.1) {
                    sprite.style.transform = 'scaleX(-1) translateZ(0)';
                } else {
                    sprite.style.transform = 'scaleX(1) translateZ(0)';
                }
            }
        }
    }

    handleCollisions(otherMiners) {
        for (const other of otherMiners) {
            if (other.pid === this.pid) continue;

            const dx = other.x - this.x;
            const dy = other.y - this.y;
            const distance = Math.sqrt(dx * dx + dy * dy);
            const minDistance = this.size; // Minimum distance between miners

            if (distance < minDistance && distance > 0) {
                // Push miners apart
                const pushStrength = (minDistance - distance) / 2;
                const angle = Math.atan2(dy, dx);

                this.x -= Math.cos(angle) * pushStrength;
                this.y -= Math.sin(angle) * pushStrength;

                // Keep within bounds
                this.x = Math.max(0, Math.min(this.worldWidth - this.size, this.x));
                this.y = Math.max(0, Math.min(this.worldHeight - this.size, this.y));
            }
        }
    }

    createElement(onClick) {
        const element = document.createElement('div');
        element.className = `miner-entity ${this.status}`;
        element.dataset.pid = this.pid;

        // Get icon based on status
        let icon;
        if (!this.has_terminal) {
            icon = '👻';
            element.classList.add('zombie');
        } else if (this.status === 'working') {
            icon = '⛏️';
        } else {
            icon = '😴';
        }

        element.innerHTML = `
            <div class="miner-sprite">${icon}</div>
            <div class="miner-pid">#${this.pid}</div>
            <div class="miner-tooltip">
                <strong>PID: ${this.pid}</strong><br>
                CPU: ${this.cpu_usage.toFixed(1)}%<br>
                MEM: ${(this.memory / 1024 / 1024).toFixed(0)} MB
            </div>
        `;

        // Add click handler
        element.addEventListener('click', (e) => {
            e.stopPropagation();
            onClick(this);
        });

        // Initial position with hardware acceleration
        element.style.transform = `translate(${this.x}px, ${this.y}px) translateZ(0)`;

        this.element = element;
        return element;
    }

    updateStatus(miner) {
        if (this.status !== miner.status || this.has_terminal !== miner.has_terminal) {
            this.status = miner.status;
            this.has_terminal = miner.has_terminal;
            this.speed = this.getSpeedByStatus();
            this.animationSpeed = this.getAnimationSpeedByStatus();
            this.moveInterval = this.getMoveIntervalByStatus();

            // Update visual
            if (this.element) {
                this.element.className = `miner-entity ${this.status}`;
                if (!this.has_terminal) {
                    this.element.classList.add('zombie');
                }

                let icon;
                if (!this.has_terminal) {
                    icon = '👻';
                } else if (this.status === 'working') {
                    icon = '⛏️';
                } else {
                    icon = '😴';
                }

                const sprite = this.element.querySelector('.miner-sprite');
                if (sprite) {
                    sprite.textContent = icon;
                }
            }
        }

        this.cpu_usage = miner.cpu_usage;
        this.memory = miner.memory;

        // Update tooltip
        if (this.element) {
            const tooltip = this.element.querySelector('.miner-tooltip');
            if (tooltip) {
                tooltip.innerHTML = `
                    <strong>PID: ${this.pid}</strong><br>
                    CPU: ${this.cpu_usage.toFixed(1)}%<br>
                    MEM: ${(this.memory / 1024 / 1024).toFixed(0)} MB
                `;
            }
        }
    }
}

class MetaverseWorld {
    constructor(containerElement) {
        this.container = containerElement;
        this.miners = new Map(); // PID -> MinerEntity
        this.width = containerElement.clientWidth;
        this.height = containerElement.clientHeight;
        this.lastUpdateTime = performance.now();
        this.animationFrameId = null;

        // Start animation loop
        this.startAnimationLoop();

        // Handle window resize
        window.addEventListener('resize', () => {
            this.width = containerElement.clientWidth;
            this.height = containerElement.clientHeight;

            // Update world bounds for all miners
            for (const miner of this.miners.values()) {
                miner.worldWidth = this.width;
                miner.worldHeight = this.height;
            }
        });
    }

    startAnimationLoop() {
        const animate = (currentTime) => {
            const deltaTime = currentTime - this.lastUpdateTime;
            this.lastUpdateTime = currentTime;

            // Update all miners
            const minerArray = Array.from(this.miners.values());
            for (const miner of minerArray) {
                miner.update(deltaTime, minerArray);
            }

            this.animationFrameId = requestAnimationFrame(animate);
        };

        this.animationFrameId = requestAnimationFrame(animate);
    }

    stopAnimationLoop() {
        if (this.animationFrameId) {
            cancelAnimationFrame(this.animationFrameId);
            this.animationFrameId = null;
        }
    }

    updateMiners(minersData, onMinerClick) {
        const currentPids = new Set(minersData.map(m => m.pid));

        // Remove miners that no longer exist
        for (const [pid, minerEntity] of this.miners.entries()) {
            if (!currentPids.has(pid)) {
                if (minerEntity.element) {
                    minerEntity.element.remove();
                }
                this.miners.delete(pid);
            }
        }

        // Add or update miners
        for (const minerData of minersData) {
            if (this.miners.has(minerData.pid)) {
                // Update existing miner
                const miner = this.miners.get(minerData.pid);
                miner.updateStatus(minerData);
            } else {
                // Create new miner
                const miner = new MinerEntity(minerData, this.width, this.height);
                const element = miner.createElement(onMinerClick);
                this.container.appendChild(element);
                this.miners.set(minerData.pid, miner);
            }
        }
    }

    getMiner(pid) {
        return this.miners.get(pid);
    }

    destroy() {
        this.stopAnimationLoop();
        this.miners.clear();
    }
}

// Export for use in app.js
window.MetaverseWorld = MetaverseWorld;
