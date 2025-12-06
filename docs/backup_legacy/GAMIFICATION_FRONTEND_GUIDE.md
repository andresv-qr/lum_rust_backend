# 🎮 **GUÍA DE IMPLEMENTACIÓN FRONTEND - SISTEMA DE GAMIFICACIÓN**

## 📋 **Tabla de Contenidos**

1. [Visión General del Sistema](#visión-general-del-sistema)
2. [Arquitectura de Integración](#arquitectura-de-integración)
3. [Mecánicas de Gamificación](#mecánicas-de-gamificación)
4. [Implementación por Pantallas](#implementación-por-pantallas)
5. [Componentes UI Reutilizables](#componentes-ui-reutilizables)
6. [Estados y Gestión de Datos](#estados-y-gestión-de-datos)
7. [Animaciones y Feedback Visual](#animaciones-y-feedback-visual)
8. [Notificaciones y Alerts](#notificaciones-y-alerts)
9. [Testing y QA](#testing-y-qa)
10. [Métricas y Analytics](#métricas-y-analytics)

---

## 🎯 **Visión General del Sistema**

### **Filosofía de Diseño UX**
El sistema de gamificación está diseñado para ser **no intrusivo** pero **altamente motivador**. Cada interacción debe sentirse como un logro natural, no como una distracción.

### **Principios Fundamentales**
- ✅ **Progreso Visible:** El usuario siempre sabe dónde está y hacia dónde va
- ✅ **Recompensas Inmediatas:** Feedback instantáneo en cada acción
- ✅ **Descubrimiento Progresivo:** Nuevas mecánicas se revelan gradualmente
- ✅ **Personalización:** Experiencia adaptada al comportamiento del usuario
- ✅ **Competencia Saludable:** Elementos sociales que motivan sin frustrar

---

## 🏗️ **Arquitectura de Integración**

### **1. Estructura de Servicios**

```typescript
// services/gamification.service.ts
class GamificationService {
  private apiClient: ApiClient;
  private cacheManager: CacheManager;
  private eventBus: EventBus;

  // Core API methods
  async trackAction(action: ActionType, metadata?: object): Promise<GamificationResponse>
  async getUserDashboard(): Promise<UserDashboard>
  async getUserMissions(): Promise<Mission[]>
  async getActiveEvents(): Promise<Event[]>
  async getUserAchievements(): Promise<Achievement[]>
  async getLeaderboard(limit?: number): Promise<LeaderboardEntry[]>
  
  // Real-time updates
  subscribeToUpdates(userId: number, callback: (update: GamificationUpdate) => void)
  unsubscribeFromUpdates(userId: number)
}
```

### **2. Estado Global**

```typescript
// store/gamification.store.ts
interface GamificationState {
  user: {
    lumis: number;
    level: UserLevel;
    streaks: Record<string, Streak>;
    activeMissions: Mission[];
    recentAchievements: Achievement[];
  };
  events: {
    activeEvents: Event[];
    happeningNow: Event[];
  };
  ui: {
    showCelebration: boolean;
    pendingRewards: Reward[];
    lastActionFeedback: ActionFeedback | null;
  };
  cache: {
    dashboard: UserDashboard | null;
    lastUpdate: Date | null;
  };
}
```

### **3. Event Bus para Comunicación**

```typescript
// events/gamification.events.ts
enum GamificationEvents {
  ACTION_TRACKED = 'gamification:action_tracked',
  REWARD_EARNED = 'gamification:reward_earned',
  LEVEL_UP = 'gamification:level_up',
  STREAK_MILESTONE = 'gamification:streak_milestone',
  ACHIEVEMENT_UNLOCKED = 'gamification:achievement_unlocked',
  MISSION_COMPLETED = 'gamification:mission_completed',
  EVENT_STARTED = 'gamification:event_started',
  LEADERBOARD_UPDATED = 'gamification:leaderboard_updated'
}
```

---

## 🎮 **Mecánicas de Gamificación**

### **1. Sistema de Puntos (Lumis) 💎**

#### **Implementación Frontend:**
```typescript
interface LumisDisplay {
  current: number;
  earned: number;
  animation: 'none' | 'earning' | 'spending';
  source?: string; // "daily_login", "invoice_upload", etc.
}

// Componente LumisCounter
const LumisCounter: React.FC<{
  lumis: number;
  showEarned?: boolean;
  earnedAmount?: number;
  size?: 'small' | 'medium' | 'large';
}> = ({ lumis, showEarned, earnedAmount, size = 'medium' }) => {
  const [animatingCount, setAnimatingCount] = useState(lumis);
  
  useEffect(() => {
    if (showEarned && earnedAmount) {
      // Animación de contador incremental
      animateCountUp(lumis - earnedAmount, lumis, 1000);
    }
  }, [lumis, earnedAmount]);

  return (
    <div className={`lumis-counter lumis-counter--${size}`}>
      <div className="lumis-icon">💎</div>
      <span className="lumis-amount">{formatNumber(animatingCount)}</span>
      {showEarned && earnedAmount > 0 && (
        <span className="lumis-earned">+{formatNumber(earnedAmount)}</span>
      )}
    </div>
  );
};
```

#### **Ubicaciones en UI:**
- **Header/AppBar:** Contador permanente (pequeño)
- **Dashboard:** Contador principal (grande) con animaciones
- **Pantalla de Recompensas:** Detalle de transacciones
- **Modal de Acción:** Feedback inmediato (+25 Lumis)

---

### **2. Sistema de Niveles 📈**

#### **Componente de Progreso:**
```typescript
const LevelProgress: React.FC<{
  currentLevel: UserLevel;
  currentXP: number;
  nextLevelXP: number;
  showDetails?: boolean;
}> = ({ currentLevel, currentXP, nextLevelXP, showDetails = false }) => {
  const progress = (currentXP / nextLevelXP) * 100;
  
  return (
    <div className="level-progress">
      <div className="level-badge">
        <img src={currentLevel.iconUrl} alt={currentLevel.name} />
        <span className="level-number">{currentLevel.number}</span>
      </div>
      
      <div className="progress-container">
        <div className="level-name" style={{ color: currentLevel.color }}>
          {currentLevel.name}
        </div>
        <div className="progress-bar">
          <div 
            className="progress-fill" 
            style={{ 
              width: `${progress}%`,
              backgroundColor: currentLevel.color 
            }}
          />
        </div>
        <div className="progress-text">
          {formatNumber(currentXP)} / {formatNumber(nextLevelXP)} XP
        </div>
      </div>

      {showDetails && (
        <div className="level-benefits">
          <h4>Beneficios de este nivel:</h4>
          <ul>
            {currentLevel.benefits.map((benefit, index) => (
              <li key={index}>{benefit}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};
```

#### **Animación de Level Up:**
```typescript
const LevelUpCelebration: React.FC<{
  newLevel: UserLevel;
  onComplete: () => void;
}> = ({ newLevel, onComplete }) => {
  return (
    <div className="level-up-overlay">
      <div className="celebration-container">
        <div className="fireworks-animation" />
        <div className="level-up-content">
          <h1>🎉 ¡NIVEL ALCANZADO! 🎉</h1>
          <div className="new-level-badge">
            <img src={newLevel.iconUrl} alt={newLevel.name} />
            <h2 style={{ color: newLevel.color }}>{newLevel.name}</h2>
          </div>
          <div className="new-benefits">
            <h3>Nuevos beneficios desbloqueados:</h3>
            <ul>
              {newLevel.benefits.map((benefit, index) => (
                <li key={index} className="benefit-item">
                  ✨ {benefit}
                </li>
              ))}
            </ul>
          </div>
          <button onClick={onComplete} className="continue-btn">
            ¡Continuar!
          </button>
        </div>
      </div>
    </div>
  );
};
```

---

### **3. Sistema de Rachas (Streaks) 🔥**

#### **Componente de Streak:**
```typescript
const StreakDisplay: React.FC<{
  streak: Streak;
  size?: 'compact' | 'detailed';
}> = ({ streak, size = 'compact' }) => {
  const getStreakIcon = (count: number) => {
    if (count >= 30) return '🔥🔥🔥';
    if (count >= 14) return '🔥🔥';
    if (count >= 7) return '🔥';
    return '⚡';
  };

  const getStreakColor = (count: number) => {
    if (count >= 30) return '#ff6b35'; // Legendary orange
    if (count >= 14) return '#ff9500'; // Epic orange
    if (count >= 7) return '#ffb700';  // Rare yellow
    return '#4CAF50'; // Common green
  };

  if (size === 'compact') {
    return (
      <div className="streak-compact">
        <span className="streak-icon">{getStreakIcon(streak.currentCount)}</span>
        <span className="streak-count">{streak.currentCount}</span>
      </div>
    );
  }

  return (
    <div className="streak-detailed">
      <div className="streak-header">
        <h3>{streak.name}</h3>
        <div className="streak-badge" style={{ backgroundColor: getStreakColor(streak.currentCount) }}>
          <span className="streak-icon">{getStreakIcon(streak.currentCount)}</span>
          <span className="streak-count">{streak.currentCount}</span>
        </div>
      </div>
      
      <div className="streak-progress">
        <div className="current-milestone">
          Racha actual: {streak.currentCount} días
        </div>
        {streak.nextMilestone && (
          <div className="next-milestone">
            Próximo bonus en {streak.nextMilestone - streak.currentCount} días
          </div>
        )}
      </div>

      <div className="streak-multiplier">
        <span>Multiplicador activo: </span>
        <strong style={{ color: getStreakColor(streak.currentCount) }}>
          x{streak.bonusMultiplier}
        </strong>
      </div>
    </div>
  );
};
```

#### **Integración con Acciones:**
```typescript
// En cada pantalla donde se pueden realizar acciones
const handleUserAction = async (actionType: ActionType, metadata?: object) => {
  try {
    // Mostrar loading
    setIsProcessing(true);
    
    // Llamar API
    const response = await gamificationService.trackAction(actionType, metadata);
    
    // Procesar respuesta
    if (response.success) {
      // Actualizar estado global
      updateGamificationState(response.data);
      
      // Mostrar feedback inmediato
      showActionFeedback({
        lumisEarned: response.data.lumisEarned,
        xpEarned: response.data.xpEarned,
        streaksUpdated: response.data.streaks,
        achievementsUnlocked: response.data.achievementsUnlocked
      });
      
      // Verificar si hay celebraciones especiales
      if (response.data.levelUp) {
        showLevelUpCelebration(response.data.newLevel);
      }
      
      if (response.data.achievementsUnlocked.length > 0) {
        showAchievementUnlocked(response.data.achievementsUnlocked);
      }
    }
  } catch (error) {
    console.error('Error tracking action:', error);
    showErrorMessage('No se pudo registrar la acción');
  } finally {
    setIsProcessing(false);
  }
};
```

---

### **4. Sistema de Misiones 🎯**

#### **Componente de Misión:**
```typescript
const MissionCard: React.FC<{
  mission: Mission;
  onClaim?: (missionId: string) => void;
}> = ({ mission, onClaim }) => {
  const progressPercentage = (mission.currentProgress / mission.targetCount) * 100;
  
  const getMissionIcon = (type: string) => {
    const icons = {
      daily: '📅',
      weekly: '📊',
      monthly: '🏆',
      special: '⭐'
    };
    return icons[type] || '🎯';
  };

  const getMissionColor = (type: string) => {
    const colors = {
      daily: '#4CAF50',
      weekly: '#2196F3',
      monthly: '#9C27B0',
      special: '#FF9800'
    };
    return colors[type] || '#757575';
  };

  return (
    <div className={`mission-card mission-card--${mission.status}`}>
      <div className="mission-header">
        <div className="mission-icon" style={{ backgroundColor: getMissionColor(mission.type) }}>
          {getMissionIcon(mission.type)}
        </div>
        <div className="mission-info">
          <h3 className="mission-name">{mission.name}</h3>
          <p className="mission-description">{mission.description}</p>
        </div>
        <div className="mission-reward">
          <span className="reward-amount">💎 {formatNumber(mission.rewardLumis)}</span>
        </div>
      </div>

      <div className="mission-progress">
        <div className="progress-bar">
          <div 
            className="progress-fill" 
            style={{ 
              width: `${progressPercentage}%`,
              backgroundColor: getMissionColor(mission.type)
            }}
          />
        </div>
        <div className="progress-text">
          {mission.currentProgress} / {mission.targetCount}
          {mission.dueDate && (
            <span className="due-date">
              Vence: {formatDate(mission.dueDate)}
            </span>
          )}
        </div>
      </div>

      {mission.status === 'completed' && onClaim && (
        <button 
          className="claim-button"
          onClick={() => onClaim(mission.missionCode)}
        >
          ¡Reclamar Recompensa!
        </button>
      )}
    </div>
  );
};
```

#### **Panel de Misiones:**
```typescript
const MissionsPanel: React.FC = () => {
  const [missions, setMissions] = useState<Mission[]>([]);
  const [filter, setFilter] = useState<'all' | 'active' | 'completed'>('all');
  
  useEffect(() => {
    loadUserMissions();
  }, []);

  const filteredMissions = missions.filter(mission => {
    if (filter === 'all') return true;
    if (filter === 'active') return mission.status === 'active';
    if (filter === 'completed') return mission.status === 'completed';
    return true;
  });

  return (
    <div className="missions-panel">
      <div className="missions-header">
        <h2>🎯 Mis Misiones</h2>
        <div className="mission-filters">
          <button 
            className={filter === 'all' ? 'active' : ''}
            onClick={() => setFilter('all')}
          >
            Todas ({missions.length})
          </button>
          <button 
            className={filter === 'active' ? 'active' : ''}
            onClick={() => setFilter('active')}
          >
            Activas ({missions.filter(m => m.status === 'active').length})
          </button>
          <button 
            className={filter === 'completed' ? 'active' : ''}
            onClick={() => setFilter('completed')}
          >
            Completadas ({missions.filter(m => m.status === 'completed').length})
          </button>
        </div>
      </div>

      <div className="missions-list">
        {filteredMissions.map(mission => (
          <MissionCard 
            key={mission.missionCode} 
            mission={mission}
            onClaim={handleClaimMission}
          />
        ))}
      </div>
    </div>
  );
};
```

---

### **5. Happy Hours y Eventos ⚡**

#### **Componente de Evento Activo:**
```typescript
const ActiveEventBanner: React.FC<{
  event: Event;
  onDismiss?: () => void;
}> = ({ event, onDismiss }) => {
  const [timeRemaining, setTimeRemaining] = useState(event.endsInMinutes * 60);
  
  useEffect(() => {
    const timer = setInterval(() => {
      setTimeRemaining(prev => {
        if (prev <= 0) {
          clearInterval(timer);
          return 0;
        }
        return prev - 1;
      });
    }, 1000);

    return () => clearInterval(timer);
  }, []);

  const formatTimeRemaining = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m ${secs}s`;
  };

  return (
    <div className="active-event-banner">
      <div className="event-content">
        <div className="event-icon">⚡</div>
        <div className="event-info">
          <h3>{event.name}</h3>
          <p>{event.description}</p>
          <div className="event-multiplier">
            Multiplicador: <strong>x{event.multiplier}</strong>
          </div>
        </div>
        <div className="event-timer">
          <div className="timer-label">Termina en:</div>
          <div className="timer-value">{formatTimeRemaining(timeRemaining)}</div>
        </div>
      </div>
      {onDismiss && (
        <button className="dismiss-button" onClick={onDismiss}>×</button>
      )}
    </div>
  );
};
```

#### **Integración con Acciones:**
```typescript
// Hook para verificar eventos activos
const useActiveEvents = () => {
  const [activeEvents, setActiveEvents] = useState<Event[]>([]);
  
  useEffect(() => {
    const loadActiveEvents = async () => {
      const events = await gamificationService.getActiveEvents();
      setActiveEvents(events.filter(e => e.isActiveNow));
    };
    
    loadActiveEvents();
    const interval = setInterval(loadActiveEvents, 60000); // Check every minute
    
    return () => clearInterval(interval);
  }, []);
  
  const getActiveMultiplier = () => {
    return activeEvents.reduce((max, event) => 
      Math.max(max, event.multiplier), 1.0
    );
  };
  
  return { activeEvents, getActiveMultiplier };
};

// En componentes de acción
const ActionButton: React.FC<{
  action: ActionType;
  onAction: () => void;
}> = ({ action, onAction }) => {
  const { activeEvents, getActiveMultiplier } = useActiveEvents();
  const multiplier = getActiveMultiplier();
  const hasActiveEvent = multiplier > 1.0;
  
  return (
    <button 
      className={`action-button ${hasActiveEvent ? 'action-button--boosted' : ''}`}
      onClick={onAction}
    >
      <span className="action-text">Realizar Acción</span>
      {hasActiveEvent && (
        <span className="multiplier-badge">
          ⚡ x{multiplier}
        </span>
      )}
    </button>
  );
};
```

---

### **6. Sistema de Logros 🏅**

#### **Componente de Achievement:**
```typescript
const AchievementBadge: React.FC<{
  achievement: Achievement;
  size?: 'small' | 'medium' | 'large';
  showProgress?: boolean;
}> = ({ achievement, size = 'medium', showProgress = false }) => {
  const getDifficultyColor = (difficulty: string) => {
    const colors = {
      bronze: '#CD7F32',
      silver: '#C0C0C0',
      gold: '#FFD700',
      platinum: '#E5E4E2',
      legendary: '#B76E79'
    };
    return colors[difficulty] || '#757575';
  };

  const getDifficultyIcon = (difficulty: string) => {
    const icons = {
      bronze: '🥉',
      silver: '🥈',
      gold: '🥇',
      platinum: '💎',
      legendary: '👑'
    };
    return icons[difficulty] || '🏅';
  };

  return (
    <div className={`achievement-badge achievement-badge--${size} ${achievement.isUnlocked ? 'unlocked' : 'locked'}`}>
      <div 
        className="achievement-icon"
        style={{ borderColor: getDifficultyColor(achievement.difficulty) }}
      >
        {achievement.isUnlocked ? getDifficultyIcon(achievement.difficulty) : '🔒'}
      </div>
      
      <div className="achievement-info">
        <h4 className="achievement-name">{achievement.name}</h4>
        <p className="achievement-description">{achievement.description}</p>
        
        {achievement.isUnlocked && achievement.unlockedAt && (
          <div className="unlock-date">
            Desbloqueado: {formatDate(achievement.unlockedAt)}
          </div>
        )}
        
        {!achievement.isUnlocked && showProgress && achievement.progress && (
          <div className="achievement-progress">
            <div className="progress-bar">
              <div 
                className="progress-fill"
                style={{ width: `${achievement.progress.percentage}%` }}
              />
            </div>
            <span className="progress-text">
              {achievement.progress.current} / {achievement.progress.target}
            </span>
          </div>
        )}
        
        <div className="achievement-reward">
          💎 {formatNumber(achievement.rewardLumis)} Lumis
        </div>
      </div>
    </div>
  );
};
```

#### **Modal de Logro Desbloqueado:**
```typescript
const AchievementUnlockedModal: React.FC<{
  achievements: Achievement[];
  onClose: () => void;
}> = ({ achievements, onClose }) => {
  const [currentIndex, setCurrentIndex] = useState(0);
  
  const handleNext = () => {
    if (currentIndex < achievements.length - 1) {
      setCurrentIndex(currentIndex + 1);
    } else {
      onClose();
    }
  };

  const currentAchievement = achievements[currentIndex];

  return (
    <div className="achievement-modal-overlay">
      <div className="achievement-modal">
        <div className="achievement-celebration">
          <div className="sparkles-animation" />
          <h1>🎉 ¡LOGRO DESBLOQUEADO! 🎉</h1>
        </div>
        
        <div className="achievement-showcase">
          <AchievementBadge 
            achievement={currentAchievement}
            size="large"
          />
        </div>
        
        <div className="achievement-details">
          <h2>{currentAchievement.name}</h2>
          <p>{currentAchievement.description}</p>
          <div className="reward-earned">
            <span>Recompensa ganada:</span>
            <strong>💎 {formatNumber(currentAchievement.rewardLumis)} Lumis</strong>
          </div>
        </div>
        
        <div className="modal-actions">
          {achievements.length > 1 && (
            <div className="achievement-counter">
              {currentIndex + 1} de {achievements.length}
            </div>
          )}
          <button onClick={handleNext} className="continue-button">
            {currentIndex < achievements.length - 1 ? 'Siguiente' : '¡Genial!'}
          </button>
        </div>
      </div>
    </div>
  );
};
```

---

### **7. Leaderboard 🏆**

#### **Componente de Leaderboard:**
```typescript
const Leaderboard: React.FC<{
  entries: LeaderboardEntry[];
  currentUserId: number;
  title?: string;
}> = ({ entries, currentUserId, title = "🏆 Ranking General" }) => {
  const getRankIcon = (rank: number) => {
    if (rank === 1) return '🥇';
    if (rank === 2) return '🥈';
    if (rank === 3) return '🥉';
    return `#${rank}`;
  };

  const getRankClass = (rank: number) => {
    if (rank <= 3) return 'top-three';
    if (rank <= 10) return 'top-ten';
    return 'regular';
  };

  const currentUserEntry = entries.find(entry => entry.userId === currentUserId);
  const currentUserRank = currentUserEntry?.rank || null;

  return (
    <div className="leaderboard">
      <h2 className="leaderboard-title">{title}</h2>
      
      {currentUserRank && currentUserRank > 10 && (
        <div className="current-user-rank">
          <div className="rank-info">
            Tu posición actual: <strong>#{currentUserRank}</strong>
          </div>
          <LeaderboardEntry 
            entry={currentUserEntry}
            isCurrentUser={true}
            showRankChange={false}
          />
        </div>
      )}
      
      <div className="leaderboard-list">
        {entries.slice(0, 50).map((entry, index) => (
          <LeaderboardEntry
            key={entry.userId}
            entry={entry}
            isCurrentUser={entry.userId === currentUserId}
            className={getRankClass(entry.rank)}
          />
        ))}
      </div>
      
      {entries.length === 0 && (
        <div className="empty-leaderboard">
          <p>🚀 ¡Sé el primero en aparecer en el ranking!</p>
        </div>
      )}
    </div>
  );
};

const LeaderboardEntry: React.FC<{
  entry: LeaderboardEntry;
  isCurrentUser: boolean;
  className?: string;
}> = ({ entry, isCurrentUser, className = '' }) => {
  return (
    <div className={`leaderboard-entry ${className} ${isCurrentUser ? 'current-user' : ''}`}>
      <div className="rank">
        {getRankIcon(entry.rank)}
      </div>
      
      <div className="user-info">
        <div className="username">{entry.username}</div>
        <div className="level-info">
          {entry.levelName}
        </div>
      </div>
      
      <div className="user-stats">
        <div className="lumis-count">
          💎 {formatNumber(entry.totalLumis)}
        </div>
      </div>
      
      {isCurrentUser && (
        <div className="current-user-badge">
          Tú
        </div>
      )}
    </div>
  );
};
```

---

## 📱 **Implementación por Pantallas**

### **1. Dashboard Principal**

```typescript
const GamificationDashboard: React.FC = () => {
  const [dashboardData, setDashboardData] = useState<UserDashboard | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const { activeEvents } = useActiveEvents();

  useEffect(() => {
    loadDashboardData();
  }, []);

  const loadDashboardData = async () => {
    try {
      const data = await gamificationService.getUserDashboard();
      setDashboardData(data);
    } catch (error) {
      console.error('Error loading dashboard:', error);
    } finally {
      setIsLoading(false);
    }
  };

  if (isLoading) {
    return <DashboardSkeleton />;
  }

  return (
    <div className="gamification-dashboard">
      {/* Active Events Banner */}
      {activeEvents.length > 0 && (
        <div className="active-events">
          {activeEvents.map(event => (
            <ActiveEventBanner key={event.eventCode} event={event} />
          ))}
        </div>
      )}

      {/* User Progress Summary */}
      <div className="progress-summary">
        <div className="lumis-summary">
          <LumisCounter 
            lumis={dashboardData.totalLumis}
            size="large"
          />
        </div>
        
        <div className="level-summary">
          <LevelProgress 
            currentLevel={dashboardData.currentLevel}
            currentXP={dashboardData.currentXP}
            nextLevelXP={dashboardData.nextLevelXP}
          />
        </div>
      </div>

      {/* Quick Stats Grid */}
      <div className="quick-stats-grid">
        <StatCard
          icon="🔥"
          title="Racha Diaria"
          value={dashboardData.activeStreaks.dailyLogin?.current || 0}
          subtitle="días consecutivos"
        />
        
        <StatCard
          icon="🎯"
          title="Misiones Activas"
          value={dashboardData.activeMissionsCount}
          subtitle="en progreso"
        />
        
        <StatCard
          icon="🏅"
          title="Logros"
          value={dashboardData.totalAchievements}
          subtitle="desbloqueados"
        />
        
        <StatCard
          icon="🏆"
          title="Ranking"
          value={dashboardData.bestRankThisWeek || '-'}
          subtitle="mejor posición"
        />
      </div>

      {/* Recent Activity */}
      <div className="recent-activity">
        <h3>📈 Actividad Reciente</h3>
        <div className="activity-list">
          {dashboardData.recentActivity.map((activity, index) => (
            <ActivityItem key={index} activity={activity} />
          ))}
        </div>
      </div>

      {/* Quick Actions */}
      <div className="quick-actions">
        <h3>⚡ Acciones Rápidas</h3>
        <div className="actions-grid">
          <QuickActionButton
            icon="📄"
            label="Subir Factura"
            action={() => navigateToInvoiceUpload()}
            lumisReward={25}
          />
          <QuickActionButton
            icon="📋"
            label="Completar Encuesta"
            action={() => navigateToSurveys()}
            lumisReward={50}
          />
          <QuickActionButton
            icon="🎯"
            label="Ver Misiones"
            action={() => navigateToMissions()}
          />
        </div>
      </div>
    </div>
  );
};
```

### **2. Pantalla de Subida de Facturas (con Gamificación)**

```typescript
const InvoiceUploadScreen: React.FC = () => {
  const [isUploading, setIsUploading] = useState(false);
  const [showRewardFeedback, setShowRewardFeedback] = useState(false);
  const [rewardData, setRewardData] = useState<ActionFeedback | null>(null);
  const { getActiveMultiplier } = useActiveEvents();

  const handleInvoiceUpload = async (file: File) => {
    setIsUploading(true);
    
    try {
      // 1. Subir la factura
      const uploadResult = await invoiceService.uploadInvoice(file);
      
      if (uploadResult.success) {
        // 2. Registrar la acción en gamificación
        const gamificationResult = await gamificationService.trackAction(
          'invoice_upload',
          {
            invoice_id: uploadResult.data.invoiceId,
            file_size: file.size,
            upload_method: 'manual'
          }
        );
        
        // 3. Mostrar feedback de recompensa
        if (gamificationResult.success) {
          setRewardData({
            lumisEarned: gamificationResult.data.lumisEarned,
            xpEarned: gamificationResult.data.xpEarned,
            streaksUpdated: gamificationResult.data.streaks,
            achievements: gamificationResult.data.achievementsUnlocked,
            message: gamificationResult.data.message
          });
          setShowRewardFeedback(true);
        }
        
        // 4. Navegación o acciones adicionales
        showSuccessMessage('¡Factura subida exitosamente!');
      }
    } catch (error) {
      console.error('Error uploading invoice:', error);
      showErrorMessage('Error al subir la factura');
    } finally {
      setIsUploading(false);
    }
  };

  return (
    <div className="invoice-upload-screen">
      {/* Header con info de recompensa */}
      <div className="upload-header">
        <h1>📄 Subir Factura</h1>
        <div className="reward-info">
          <span className="base-reward">💎 +25 Lumis base</span>
          {getActiveMultiplier() > 1.0 && (
            <span className="multiplier-bonus">
              ⚡ x{getActiveMultiplier()} (Happy Hour)
            </span>
          )}
        </div>
      </div>

      {/* Upload Component */}
      <FileUploader
        onUpload={handleInvoiceUpload}
        isLoading={isUploading}
        acceptedTypes={['image/*', 'application/pdf']}
      />

      {/* Reward Feedback Modal */}
      {showRewardFeedback && rewardData && (
        <RewardFeedbackModal
          rewardData={rewardData}
          onClose={() => setShowRewardFeedback(false)}
        />
      )}

      {/* Streak Progress (si es relevante) */}
      <div className="upload-progress-info">
        <h3>🔥 Progreso de Racha</h3>
        <p>Sube facturas diariamente para mantener tu racha activa</p>
        {/* Mostrar progreso de streak de facturas */}
      </div>
    </div>
  );
};
```

### **3. Pantalla de Encuestas**

```typescript
const SurveyScreen: React.FC<{ surveyId: string }> = ({ surveyId }) => {
  const [survey, setSurvey] = useState<Survey | null>(null);
  const [currentQuestion, setCurrentQuestion] = useState(0);
  const [answers, setAnswers] = useState<Record<string, any>>({});
  const [isCompleting, setIsCompleting] = useState(false);

  const handleSurveyComplete = async () => {
    setIsCompleting(true);
    
    try {
      // 1. Enviar respuestas de la encuesta
      const surveyResult = await surveyService.submitSurvey(surveyId, answers);
      
      if (surveyResult.success) {
        // 2. Registrar acción de gamificación
        const gamificationResult = await gamificationService.trackAction(
          'survey_complete',
          {
            survey_id: surveyId,
            completion_time: Date.now() - startTime,
            question_count: survey.questions.length
          }
        );
        
        // 3. Mostrar celebración de recompensa
        if (gamificationResult.success) {
          showRewardCelebration(gamificationResult.data);
        }
        
        // 4. Verificar misiones completadas
        checkMissionProgress();
      }
    } catch (error) {
      console.error('Error completing survey:', error);
    } finally {
      setIsCompleting(false);
    }
  };

  return (
    <div className="survey-screen">
      {/* Progress Bar con incentivo */}
      <div className="survey-progress">
        <div className="progress-bar">
          <div 
            className="progress-fill"
            style={{ width: `${(currentQuestion / survey.questions.length) * 100}%` }}
          />
        </div>
        <div className="progress-info">
          <span>Pregunta {currentQuestion + 1} de {survey.questions.length}</span>
          <span className="reward-preview">💎 +50 Lumis al completar</span>
        </div>
      </div>

      {/* Survey Content */}
      <SurveyQuestion
        question={survey.questions[currentQuestion]}
        onAnswer={(answer) => handleAnswer(currentQuestion, answer)}
        value={answers[currentQuestion]}
      />

      {/* Navigation */}
      <div className="survey-navigation">
        {currentQuestion > 0 && (
          <button onClick={() => setCurrentQuestion(currentQuestion - 1)}>
            Anterior
          </button>
        )}
        
        {currentQuestion < survey.questions.length - 1 ? (
          <button 
            onClick={() => setCurrentQuestion(currentQuestion + 1)}
            disabled={!answers[currentQuestion]}
          >
            Siguiente
          </button>
        ) : (
          <button 
            onClick={handleSurveyComplete}
            disabled={!answers[currentQuestion] || isCompleting}
            className="complete-button"
          >
            {isCompleting ? 'Completando...' : '¡Completar Encuesta!'}
          </button>
        )}
      </div>
    </div>
  );
};
```

---

## 🎨 **Componentes UI Reutilizables**

### **1. Feedback de Recompensa Inmediata**

```typescript
const RewardFeedbackModal: React.FC<{
  rewardData: ActionFeedback;
  onClose: () => void;
}> = ({ rewardData, onClose }) => {
  return (
    <div className="reward-feedback-overlay">
      <div className="reward-feedback-modal">
        <div className="reward-celebration">
          <div className="confetti-animation" />
          <h2>🎉 ¡Recompensa Ganada! 🎉</h2>
        </div>

        <div className="rewards-earned">
          {rewardData.lumisEarned > 0 && (
            <div className="reward-item lumis">
              <span className="reward-icon">💎</span>
              <span className="reward-amount">+{formatNumber(rewardData.lumisEarned)}</span>
              <span className="reward-label">Lumis</span>
            </div>
          )}

          {rewardData.xpEarned > 0 && (
            <div className="reward-item xp">
              <span className="reward-icon">⭐</span>
              <span className="reward-amount">+{rewardData.xpEarned}</span>
              <span className="reward-label">XP</span>
            </div>
          )}
        </div>

        {rewardData.streaksUpdated && Object.keys(rewardData.streaksUpdated).length > 0 && (
          <div className="streaks-updated">
            <h3>🔥 Rachas Actualizadas</h3>
            {Object.entries(rewardData.streaksUpdated).map(([type, streak]) => (
              <div key={type} className="streak-update">
                <span>{getStreakDisplayName(type)}: </span>
                <strong>{streak.current} días</strong>
                {streak.bonusApplied && (
                  <span className="bonus-badge">¡Bonus aplicado!</span>
                )}
              </div>
            ))}
          </div>
        )}

        {rewardData.message && (
          <div className="reward-message">
            <p>{rewardData.message}</p>
          </div>
        )}

        <button onClick={onClose} className="continue-button">
          ¡Continuar!
        </button>
      </div>
    </div>
  );
};
```

### **2. Widget Flotante de Progreso**

```typescript
const FloatingProgressWidget: React.FC = () => {
  const [isVisible, setIsVisible] = useState(false);
  const [progressData, setProgressData] = useState(null);
  const gamificationState = useGamificationState();

  useEffect(() => {
    // Mostrar widget cuando hay progreso reciente
    if (gamificationState.ui.lastActionFeedback) {
      setProgressData(gamificationState.ui.lastActionFeedback);
      setIsVisible(true);
      
      // Auto-hide después de 5 segundos
      const timer = setTimeout(() => {
        setIsVisible(false);
      }, 5000);
      
      return () => clearTimeout(timer);
    }
  }, [gamificationState.ui.lastActionFeedback]);

  if (!isVisible || !progressData) {
    return null;
  }

  return (
    <div className="floating-progress-widget">
      <div className="widget-content">
        <div className="lumis-earned">
          💎 +{progressData.lumisEarned}
        </div>
        {progressData.streakBonus && (
          <div className="streak-bonus">
            🔥 Streak Bonus!
          </div>
        )}
      </div>
      
      <button 
        className="close-widget"
        onClick={() => setIsVisible(false)}
      >
        ×
      </button>
    </div>
  );
};
```

### **3. Indicador de Multiplicador Activo**

```typescript
const MultiplierIndicator: React.FC = () => {
  const { activeEvents, getActiveMultiplier } = useActiveEvents();
  const multiplier = getActiveMultiplier();
  
  if (multiplier <= 1.0) {
    return null;
  }

  const mainEvent = activeEvents[0]; // Evento principal activo

  return (
    <div className="multiplier-indicator">
      <div className="multiplier-badge">
        <span className="multiplier-icon">⚡</span>
        <span className="multiplier-value">x{multiplier}</span>
      </div>
      
      <div className="multiplier-info">
        <div className="event-name">{mainEvent.name}</div>
        <div className="time-remaining">
          Termina en: {formatTimeRemaining(mainEvent.endsInMinutes)}
        </div>
      </div>
    </div>
  );
};
```

---

## 🔔 **Notificaciones y Alerts**

### **1. Sistema de Notificaciones In-App**

```typescript
const NotificationSystem: React.FC = () => {
  const [notifications, setNotifications] = useState<Notification[]>([]);

  useEffect(() => {
    // Suscribirse a eventos de gamificación
    const unsubscribe = gamificationService.subscribeToUpdates(
      currentUserId,
      handleGamificationUpdate
    );

    return unsubscribe;
  }, []);

  const handleGamificationUpdate = (update: GamificationUpdate) => {
    const notification = createNotificationFromUpdate(update);
    if (notification) {
      addNotification(notification);
    }
  };

  const createNotificationFromUpdate = (update: GamificationUpdate): Notification | null => {
    switch (update.type) {
      case 'mission_completed':
        return {
          id: generateId(),
          type: 'success',
          title: '🎯 ¡Misión Completada!',
          message: `Has completado "${update.data.missionName}"`,
          action: {
            label: 'Reclamar Recompensa',
            callback: () => claimMissionReward(update.data.missionId)
          },
          duration: 8000
        };

      case 'achievement_unlocked':
        return {
          id: generateId(),
          type: 'achievement',
          title: '🏅 ¡Logro Desbloqueado!',
          message: `Nuevo logro: "${update.data.achievementName}"`,
          action: {
            label: 'Ver Detalles',
            callback: () => showAchievementDetails(update.data.achievementId)
          },
          duration: 10000
        };

      case 'happy_hour_started':
        return {
          id: generateId(),
          type: 'event',
          title: '⚡ ¡Happy Hour Comenzó!',
          message: `Multiplicador x${update.data.multiplier} activo ahora`,
          action: {
            label: 'Aprovechar',
            callback: () => navigateToActions()
          },
          duration: 15000
        };

      default:
        return null;
    }
  };

  return (
    <div className="notification-system">
      {notifications.map(notification => (
        <NotificationToast
          key={notification.id}
          notification={notification}
          onDismiss={() => removeNotification(notification.id)}
        />
      ))}
    </div>
  );
};

const NotificationToast: React.FC<{
  notification: Notification;
  onDismiss: () => void;
}> = ({ notification, onDismiss }) => {
  useEffect(() => {
    if (notification.duration) {
      const timer = setTimeout(onDismiss, notification.duration);
      return () => clearTimeout(timer);
    }
  }, [notification.duration, onDismiss]);

  return (
    <div className={`notification-toast notification-toast--${notification.type}`}>
      <div className="notification-content">
        <h4 className="notification-title">{notification.title}</h4>
        <p className="notification-message">{notification.message}</p>
        
        {notification.action && (
          <button 
            className="notification-action"
            onClick={notification.action.callback}
          >
            {notification.action.label}
          </button>
        )}
      </div>
      
      <button className="notification-close" onClick={onDismiss}>
        ×
      </button>
    </div>
  );
};
```

### **2. Push Notifications (React Native)**

```typescript
// services/push-notifications.service.ts
class PushNotificationService {
  async initializePushNotifications() {
    // Configurar push notifications
    const permission = await Notifications.requestPermissionsAsync();
    
    if (permission.granted) {
      const token = await Notifications.getExpoPushTokenAsync();
      await this.registerTokenWithBackend(token.data);
    }
  }

  async scheduleDailyStreakReminder(userId: number) {
    const lastLoginTime = await this.getLastLoginTime(userId);
    const now = new Date();
    const timeSinceLastLogin = now.getTime() - lastLoginTime.getTime();
    
    // Si no ha hecho login en 18 horas, programar recordatorio
    if (timeSinceLastLogin > 18 * 60 * 60 * 1000) {
      await Notifications.scheduleNotificationAsync({
        content: {
          title: "🔥 ¡No pierdas tu racha!",
          body: "Inicia sesión para mantener tu racha diaria activa",
          data: { type: 'streak_reminder', userId }
        },
        trigger: { hour: 19, minute: 0, repeats: true }
      });
    }
  }

  async notifyMissionDeadline(mission: Mission) {
    if (!mission.dueDate) return;
    
    const hoursUntilDeadline = this.getHoursUntilDeadline(mission.dueDate);
    
    if (hoursUntilDeadline <= 24 && hoursUntilDeadline > 0) {
      await Notifications.scheduleNotificationAsync({
        content: {
          title: "⏰ ¡Misión por vencer!",
          body: `"${mission.name}" vence en ${Math.floor(hoursUntilDeadline)} horas`,
          data: { type: 'mission_deadline', missionId: mission.missionCode }
        },
        trigger: null // Enviar inmediatamente
      });
    }
  }
}
```

---

## 📊 **Testing y QA**

### **1. Tests de Componentes**

```typescript
// __tests__/components/LumisCounter.test.tsx
describe('LumisCounter Component', () => {
  it('should display current lumis count', () => {
    render(<LumisCounter lumis={1250} />);
    expect(screen.getByText('1,250')).toBeInTheDocument();
  });

  it('should animate when earning lumis', () => {
    const { rerender } = render(<LumisCounter lumis={1000} />);
    
    rerender(
      <LumisCounter 
        lumis={1025} 
        showEarned={true} 
        earnedAmount={25} 
      />
    );
    
    expect(screen.getByText('+25')).toBeInTheDocument();
  });

  it('should format large numbers correctly', () => {
    render(<LumisCounter lumis={1250000} />);
    expect(screen.getByText('1,250,000')).toBeInTheDocument();
  });
});
```

### **2. Tests de Integración**

```typescript
// __tests__/integration/gamification-flow.test.tsx
describe('Gamification Integration Flow', () => {
  it('should track action and show reward feedback', async () => {
    const mockApiResponse = {
      success: true,
      data: {
        lumisEarned: 25,
        xpEarned: 10,
        totalLumis: 1025,
        streaks: { daily_login: { current: 3 } }
      }
    };

    mockApiClient.post('/api/v4/gamification/track').mockResolvedValue(mockApiResponse);

    render(<InvoiceUploadScreen />);
    
    // Simulate file upload
    const fileInput = screen.getByTestId('file-input');
    const file = new File(['test'], 'test.pdf', { type: 'application/pdf' });
    
    fireEvent.change(fileInput, { target: { files: [file] } });
    fireEvent.click(screen.getByText('Subir Factura'));

    // Verify API call
    await waitFor(() => {
      expect(mockApiClient.post).toHaveBeenCalledWith(
        '/api/v4/gamification/track',
        expect.objectContaining({
          action: 'invoice_upload'
        })
      );
    });

    // Verify reward feedback appears
    await waitFor(() => {
      expect(screen.getByText('¡Recompensa Ganada!')).toBeInTheDocument();
      expect(screen.getByText('+25')).toBeInTheDocument();
    });
  });
});
```

### **3. Tests de Performance**

```typescript
// __tests__/performance/gamification.performance.test.tsx
describe('Gamification Performance', () => {
  it('should update dashboard within 500ms', async () => {
    const startTime = performance.now();
    
    render(<GamificationDashboard />);
    
    await waitFor(() => {
      expect(screen.getByTestId('dashboard-content')).toBeInTheDocument();
    });
    
    const endTime = performance.now();
    const renderTime = endTime - startTime;
    
    expect(renderTime).toBeLessThan(500);
  });

  it('should handle 100+ leaderboard entries efficiently', () => {
    const largeLeaderboard = generateMockLeaderboard(100);
    
    const startTime = performance.now();
    render(<Leaderboard entries={largeLeaderboard} currentUserId={1} />);
    const endTime = performance.now();
    
    expect(endTime - startTime).toBeLessThan(100);
  });
});
```

---

## 📈 **Métricas y Analytics**

### **1. Tracking de Eventos**

```typescript
// services/analytics.service.ts
class AnalyticsService {
  trackGamificationEvent(eventName: string, properties: object) {
    // Enviar a plataforma de analytics (Mixpanel, Firebase, etc.)
    this.analytics.track(`Gamification_${eventName}`, {
      ...properties,
      timestamp: new Date().toISOString(),
      user_level: this.getCurrentUserLevel(),
      total_lumis: this.getCurrentUserLumis()
    });
  }

  // Eventos específicos de gamificación
  trackRewardEarned(rewardType: string, amount: number, source: string) {
    this.trackGamificationEvent('Reward_Earned', {
      reward_type: rewardType,
      amount: amount,
      source: source
    });
  }

  trackLevelUp(oldLevel: number, newLevel: number, levelName: string) {
    this.trackGamificationEvent('Level_Up', {
      old_level: oldLevel,
      new_level: newLevel,
      level_name: levelName
    });
  }

  trackMissionCompleted(missionCode: string, completionTime: number) {
    this.trackGamificationEvent('Mission_Completed', {
      mission_code: missionCode,
      completion_time_ms: completionTime
    });
  }

  trackAchievementUnlocked(achievementCode: string, difficulty: string) {
    this.trackGamificationEvent('Achievement_Unlocked', {
      achievement_code: achievementCode,
      difficulty: difficulty
    });
  }
}
```

### **2. Métricas de Engagement**

```typescript
// hooks/useEngagementMetrics.ts
const useEngagementMetrics = () => {
  const [metrics, setMetrics] = useState<EngagementMetrics | null>(null);

  useEffect(() => {
    // Calcular métricas de engagement
    const calculateMetrics = async () => {
      const data = await analyticsService.getEngagementMetrics();
      setMetrics(data);
    };

    calculateMetrics();
  }, []);

  return {
    // Métricas de retención
    dailyActiveUsers: metrics?.dailyActiveUsers,
    weeklyRetention: metrics?.weeklyRetention,
    
    // Métricas de gamificación
    averageSessionLumis: metrics?.averageSessionLumis,
    missionCompletionRate: metrics?.missionCompletionRate,
    achievementUnlockRate: metrics?.achievementUnlockRate,
    
    // Métricas de progresión
    averageUserLevel: metrics?.averageUserLevel,
    levelUpFrequency: metrics?.levelUpFrequency,
    streakMaintenance: metrics?.streakMaintenance
  };
};
```

---

## 🚀 **Implementación Paso a Paso**

### **Fase 1: Fundación (Semana 1-2)**
1. ✅ Configurar servicios base de API
2. ✅ Implementar componentes básicos (LumisCounter, LevelProgress)
3. ✅ Integrar tracking de acciones principales
4. ✅ Desarrollar feedback básico de recompensas

### **Fase 2: Mecánicas Core (Semana 3-4)**
1. ✅ Sistema completo de streaks
2. ✅ Dashboard de gamificación
3. ✅ Sistema de misiones
4. ✅ Celebraciones y animaciones

### **Fase 3: Características Avanzadas (Semana 5-6)**
1. ✅ Sistema de logros y badges
2. ✅ Happy Hours y eventos
3. ✅ Leaderboards y competencia
4. ✅ Notificaciones push

### **Fase 4: Optimización (Semana 7-8)**
1. ✅ Performance y caching
2. ✅ Testing completo
3. ✅ Analytics y métricas
4. ✅ Documentación y handoff

---

## 💡 **Mejores Prácticas**

### **UX/UI Guidelines:**
- ✅ **Feedback Inmediato:** Toda acción debe tener respuesta visual instantánea
- ✅ **Progreso Visible:** El usuario siempre debe saber su estado actual
- ✅ **Celebraciones Moderadas:** Impactantes pero no intrusivas
- ✅ **Accesibilidad:** Soporte para lectores de pantalla y navegación por teclado

### **Performance:**
- ✅ **Lazy Loading:** Cargar componentes de gamificación bajo demanda
- ✅ **Caching Inteligente:** Cache local para datos de usuario frecuentes
- ✅ **Optimistic Updates:** Actualizar UI inmediatamente, confirmar después

### **Seguridad:**
- ✅ **Validación Client-Side:** Validar datos antes de enviar
- ✅ **Rate Limiting:** Evitar spam de acciones
- ✅ **Anti-Gaming:** Detección de comportamientos sospechosos

---

## 🎯 **Entregables Finales**

1. **📱 Aplicación Completamente Gamificada**
2. **📊 Dashboard de Engagement Metrics**
3. **🧪 Suite de Tests Automatizados**
4. **📖 Documentación de Componentes**
5. **🚀 Plan de Rollout Gradual**

---

**🎮 ¡Sistema de Gamificación Listo para Implementar! 🚀**
