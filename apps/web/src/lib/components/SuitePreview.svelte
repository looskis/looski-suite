<script lang="ts">
  import Icon from './Icon.svelte';
  let active = $state<'people' | 'board' | 'inbox'>('board');
  const views = [
    { key: 'people', label: 'Relationships', icon: 'people' },
    { key: 'board', label: 'Projects', icon: 'board' },
    { key: 'inbox', label: 'Helpdesk', icon: 'inbox' },
  ] as const;
  const columns = [
    {
      label: 'Up next',
      cards: [
        { title: 'Map the customer journey', tag: 'Planning', initials: 'AL' },
        { title: 'Gather brand assets', tag: 'Creative', initials: 'MK' },
      ],
    },
    {
      label: 'In progress',
      cards: [
        {
          title: 'Bring the new website to life',
          tag: 'Website',
          initials: 'JN',
        },
        { title: 'Prepare launch emails', tag: 'Marketing', initials: 'AL' },
      ],
    },
    {
      label: 'Done',
      cards: [
        { title: 'Meet the team', tag: 'Kickoff', initials: 'MK' },
        { title: 'Agree on the big picture', tag: 'Strategy', initials: 'JN' },
      ],
    },
  ];
</script>

<div class="suite-preview">
  <div class="preview-bar">
    <span class="preview-brand"
      ><img src="/brand/looski-mark.svg" alt="" width="25" height="25" />Studio
      North <span>/ Workspace</span></span
    ><span class="sample">Illustrative workspace</span>
  </div>
  <div class="preview-body">
    <div class="preview-sidebar">
      <span class="brand-kicker">Your tools</span>
      <div class="view-buttons" aria-label="Explore example workflows">
        {#each views as view}<button
            type="button"
            class:active={active === view.key}
            aria-pressed={active === view.key}
            onclick={() => (active = view.key)}
            ><Icon name={view.icon} size={17} />{view.label}</button
          >{/each}
      </div>
      <div class="more-tools">
        <span><Icon name="form" size={16} />Forms</span><span
          ><Icon name="calendar" size={16} />Scheduling</span
        ><span><Icon name="files" size={16} />Files</span>
      </div>
      <span class="preview-bottom"><span></span>One connected suite</span>
    </div>
    <div class="preview-content" aria-live="polite" aria-atomic="true">
      {#if active === 'board'}
        <div class="preview-title">
          <div>
            <span class="brand-kicker">Projects / Website launch</span>
            <h3>Good work, in motion.</h3>
          </div>
          <span class="team"><i>AL</i><i>MK</i><i>JN</i></span>
        </div>
        <div class="board">
          {#each columns as column, i}<div class="board-column">
              <p>
                <span class="column-dot" class:done={i === 2}
                ></span>{column.label}<span>2</span>
              </p>
              {#each column.cards as card}<div class="task-card">
                  <span class="task-tag" class:violet={i === 1}>{card.tag}</span
                  ><strong>{card.title}</strong>
                  <div>
                    <Icon name={i === 2 ? 'tasks' : 'clock'} size={14} /><span
                      >{i === 2 ? 'Completed' : 'Website launch'}</span
                    ><i>{card.initials}</i>
                  </div>
                </div>{/each}
            </div>{/each}
        </div>
      {:else if active === 'people'}
        <div class="preview-title">
          <div>
            <span class="brand-kicker">Contacts / Your relationships</span>
            <h3>People first. Context included.</h3>
          </div>
          <Icon name="people" size={24} />
        </div>
        <div class="record-list">
          {#each [{ name: 'Alex Morgan', company: 'Northstar Studio', note: 'Website discovery', initials: 'AM' }, { name: 'Jamie Chen', company: 'Fieldwork Co.', note: 'Proposal follow-up', initials: 'JC' }, { name: 'Sam Rivera', company: 'Good Company', note: 'Project kickoff', initials: 'SR' }] as person}<div
              class="record"
            >
              <i>{person.initials}</i>
              <div>
                <strong>{person.name}</strong><small>{person.company}</small>
              </div>
              <span>{person.note}</span><Icon name="arrow" size={16} />
            </div>{/each}
        </div>
        <p class="preview-note">
          Contacts, organizations, and the next conversation. Together.
        </p>
      {:else}
        <div class="preview-title">
          <div>
            <span class="brand-kicker">Helpdesk / Team inbox</span>
            <h3>Every request has a home.</h3>
          </div>
          <Icon name="inbox" size={24} />
        </div>
        <div class="record-list">
          {#each [{ title: 'A hand with getting started', name: 'Alex Morgan', status: 'Open' }, { title: 'A question about our project', name: 'Jamie Chen', status: 'In progress' }, { title: 'Thanks for the quick response!', name: 'Sam Rivera', status: 'Resolved' }] as ticket}<div
              class="record"
            >
              <Icon name="inbox" size={20} />
              <div>
                <strong>{ticket.title}</strong><small>{ticket.name}</small>
              </div>
              <span class="ticket-status">{ticket.status}</span>
            </div>{/each}
        </div>
        <p class="preview-note">
          Customer replies and internal notes, with context close at hand.
        </p>
      {/if}
    </div>
  </div>
</div>

<style>
  .suite-preview {
    color: var(--brand-ink);
    background: var(--surface-panel);
    border: 1px solid var(--brand-night-line);
    border-radius: var(--brand-radius);
    overflow: hidden;
    box-shadow: 0 24px 80px #0004;
    text-align: left;
  }
  .preview-bar {
    padding: 17px 24px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    border-bottom: 1px solid var(--brand-line);
    background: var(--brand-paper);
  }
  .preview-brand {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
    font-weight: 600;
  }
  .preview-brand > span {
    color: var(--brand-muted);
    font-weight: 400;
  }
  .sample {
    font: 9px var(--brand-mono);
    color: var(--brand-muted);
  }
  .preview-body {
    display: grid;
    grid-template-columns: 190px minmax(0, 1fr);
  }
  .preview-sidebar {
    display: flex;
    flex-direction: column;
    padding: 26px 15px 20px;
    border-right: 1px solid var(--brand-line);
  }
  .preview-sidebar > .brand-kicker {
    font-size: 9px;
    padding-left: 12px;
    color: var(--brand-muted);
    margin-bottom: 18px;
  }
  .view-buttons {
    display: grid;
    gap: 5px;
  }
  .view-buttons button {
    justify-content: flex-start;
    min-height: 40px;
    padding: 10px 12px;
    gap: 10px;
    background: transparent;
    color: var(--brand-muted);
    font-size: 12px;
    font-weight: 400;
    border: 1px solid transparent;
    box-shadow: none;
  }
  .view-buttons button:hover {
    color: var(--brand-ink);
    background: var(--surface-hover);
  }
  .view-buttons button.active {
    color: var(--brand-ink);
    background: #e7f0eb;
    border-color: #d7e5dd;
    font-weight: 500;
  }
  .more-tools {
    display: grid;
    gap: 20px;
    margin: 20px 12px 26px;
    color: var(--brand-muted);
    font-size: 12px;
  }
  .more-tools > span {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .preview-bottom {
    display: flex;
    gap: 7px;
    align-items: center;
    margin: auto 12px 0;
    font-size: 9px;
    color: var(--brand-muted);
  }
  .preview-bottom > span {
    width: 5px;
    height: 5px;
    background: #438569;
    border-radius: 50%;
  }
  .preview-content {
    min-width: 0;
    padding: 28px;
    min-height: 370px;
  }
  .preview-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 25px;
  }
  .preview-title .brand-kicker {
    font-size: 9px;
    color: var(--brand-muted);
  }
  h3 {
    font-size: 23px;
    margin: 9px 0 0;
  }
  .team {
    display: flex;
    padding-right: 4px;
  }
  i {
    font: 500 9px var(--font-family-sans);
    width: 26px;
    height: 26px;
    border-radius: 50%;
    display: inline-grid;
    place-items: center;
    background: #e8e7f1;
    flex-shrink: 0;
  }
  .team i {
    border: 2px solid white;
    width: 30px;
    height: 30px;
    margin-right: -6px;
  }
  .team i:nth-child(2) {
    background: #e2eee4;
  }
  .team i:nth-child(3) {
    background: #f1e8dc;
  }
  .board {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
  }
  .board-column > p {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 10px;
    margin: 0 0 13px;
  }
  .board-column > p > span:last-child {
    margin-left: auto;
    color: var(--brand-muted);
  }
  .column-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #b5b7c9;
  }
  .column-dot.done {
    background: #559d7c;
  }
  .task-card {
    border: 1px solid var(--brand-line);
    padding: 14px;
    border-radius: 9px;
    margin-bottom: 10px;
    box-shadow: 0 2px 3px #19232903;
  }
  .task-tag {
    display: inline-block;
    font-size: 8px;
    border-radius: 4px;
    padding: 3px 6px;
    background: #eaf1ea;
    color: #3d6650;
  }
  .task-tag.violet {
    color: #635277;
    background: #f1ecf7;
  }
  .task-card strong {
    display: block;
    font-size: 12px;
    line-height: 1.5;
    font-weight: 500;
    margin: 10px 0 16px;
  }
  .task-card > div {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--brand-muted);
    font-size: 8px;
  }
  .task-card i {
    margin-left: auto;
    width: 20px;
    height: 20px;
    font-size: 7px;
    color: var(--brand-ink);
  }
  .record-list {
    border-top: 1px solid var(--brand-line);
  }
  .record {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 20px 0;
    border-bottom: 1px solid var(--brand-line);
  }
  .record strong {
    font-size: 13px;
    font-weight: 500;
  }
  .record small {
    display: block;
    margin-top: 5px;
    font-size: 11px;
    color: var(--brand-muted);
  }
  .record > span {
    margin-left: auto;
    font-size: 10px;
    color: var(--brand-muted);
  }
  .ticket-status {
    border-radius: 20px;
    padding: 5px 9px;
    background: #edf3ef;
  }
  .preview-note {
    font-size: 11px;
    color: var(--brand-muted);
    margin-bottom: 0;
  }
  @media (max-width: 900px) {
    .preview-body {
      grid-template-columns: 160px minmax(0, 1fr);
    }
    .preview-content {
      padding: 22px 18px;
    }
    .task-card {
      padding: 11px;
    }
    .task-card > div > span {
      display: none;
    }
  }
  @media (max-width: 640px) {
    .preview-bar {
      padding: 14px;
    }
    .preview-brand {
      font-size: 12px;
    }
    .preview-brand > span {
      display: none;
    }
    .sample {
      font-size: 8px;
    }
    .preview-body {
      display: block;
    }
    .preview-sidebar {
      border-right: 0;
      border-bottom: 1px solid var(--brand-line);
      padding: 10px;
    }
    .preview-sidebar > .brand-kicker,
    .more-tools,
    .preview-bottom {
      display: none;
    }
    .view-buttons {
      display: flex;
      gap: 3px;
    }
    .view-buttons button {
      flex: 1;
      justify-content: center;
      padding: 8px 6px;
      font-size: 10px;
      gap: 5px;
    }
    .preview-content {
      padding: 20px 14px;
      min-height: 350px;
    }
    h3 {
      font-size: 21px;
    }
    .team {
      display: none;
    }
    .board {
      gap: 8px;
    }
    .task-card {
      padding: 9px;
    }
    .task-card strong {
      font-size: 10px;
      min-height: 45px;
    }
    .task-tag {
      font-size: 7px;
      padding: 3px;
    }
    .board-column > p {
      font-size: 9px;
      gap: 4px;
    }
    .record {
      gap: 10px;
    }
    .record > span:not(.ticket-status) {
      display: none;
    }
    .record strong {
      font-size: 11px;
    }
  }
</style>
