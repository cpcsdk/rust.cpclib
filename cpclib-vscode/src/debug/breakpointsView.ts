import * as vscode from 'vscode';

/**
 * Every kind of breakpoint a CPC debug session holds, each explained - the
 * editor's own Breakpoints pane shows the red dots alone, when a session also
 * has the program's BREAKPOINT directives (execution, memory, I/O), data
 * breakpoints, watches, the stop on entry, and on bemu what the emulator
 * itself holds, hits counted.
 *
 * The adapter is the source of truth: `cpclib/breakpoints` answers the kinds,
 * their explanation and their entries; `cpclib/emulatorBreakpoints` (bemu)
 * what the emulator really has armed.
 */

/** One breakpoint, as `cpclib/breakpoints` answers it. */
interface Entry {
    label: string;
    detail: string;
    /** `armed`, `partial` (part of it not applied) or `inactive`. */
    status: 'armed' | 'partial' | 'inactive';
    address?: string;
    note?: string;
    path?: string;
    line?: number;
    log?: boolean;
    conditional?: boolean;
}

interface Kind {
    id: string;
    title: string;
    description: string;
    entries: Entry[];
}

/** One of the emulator's own breakpoints (`cpclib/emulatorBreakpoints`). */
interface EmulatorBreakpoint {
    owner: string;
    label: string;
    kind: string;
    access?: string;
    run?: string;
    addr?: number;
    condition?: string | null;
    hits?: number;
    after?: number;
}

interface EmulatorList {
    advanced: EmulatorBreakpoint[];
    addresses: string[];
    raster: { x: number; nop: number; line?: number }[];
}

type Node =
    | { type: 'kind'; kind: Kind }
    | { type: 'entry'; kind: Kind; entry: Entry };

const EMULATOR_KIND_ID = 'emulator';

export class BreakpointsView implements vscode.TreeDataProvider<Node> {
    private kinds: Kind[] = [];
    private readonly changed = new vscode.EventEmitter<Node | undefined>();
    readonly onDidChangeTreeData = this.changed.event;
    private pending: ReturnType<typeof setTimeout> | undefined;

    /** Ask the session again, soon: several changes in a row ask once. */
    refresh(): void {
        if (this.pending) { clearTimeout(this.pending); }
        this.pending = setTimeout(() => { this.pending = undefined; void this.load(); }, 150);
    }

    clear(): void {
        this.kinds = [];
        this.changed.fire(undefined);
    }

    private async load(): Promise<void> {
        const session = vscode.debug.activeDebugSession;
        if (!session || session.type !== 'basm') { this.clear(); return; }
        let kinds: Kind[] = [];
        try {
            const answer = await session.customRequest('cpclib/breakpoints');
            kinds = (answer?.kinds ?? []) as Kind[];
        } catch {
            // A BASIC session or an older adapter: nothing to show.
        }
        try {
            const list = await session.customRequest('cpclib/emulatorBreakpoints') as EmulatorList;
            kinds.push(emulatorKind(list));
        } catch {
            // This emulator does not list what it holds.
        }
        this.kinds = kinds;
        this.changed.fire(undefined);
    }

    getChildren(node?: Node): Node[] {
        if (!node) { return this.kinds.map(kind => ({ type: 'kind', kind })); }
        if (node.type === 'kind') {
            return node.kind.entries.map(entry => ({ type: 'entry', kind: node.kind, entry }));
        }
        return [];
    }

    getTreeItem(node: Node): vscode.TreeItem {
        if (node.type === 'kind') {
            const { kind } = node;
            const item = new vscode.TreeItem(
                kind.title,
                kind.entries.length > 0
                    ? vscode.TreeItemCollapsibleState.Expanded
                    : vscode.TreeItemCollapsibleState.None,
            );
            item.description = kind.entries.length > 0 ? String(kind.entries.length) : 'none';
            item.tooltip = new vscode.MarkdownString(`**${kind.title}**\n\n${kind.description}`);
            item.iconPath = new vscode.ThemeIcon(KIND_ICONS[kind.id] ?? 'debug-breakpoint');
            item.contextValue = 'cpclibBreakpointKind';
            return item;
        }
        const { entry, kind } = node;
        const item = new vscode.TreeItem(entry.label, vscode.TreeItemCollapsibleState.None);
        item.description = entry.detail;
        const tooltip = new vscode.MarkdownString();
        tooltip.appendMarkdown(`**${entry.label}** — ${kind.title}\n\n`);
        tooltip.appendText(entry.detail);
        tooltip.appendMarkdown(`\n\n${STATUS_TEXT[entry.status]}`);
        if (entry.note) { tooltip.appendMarkdown('\n\n'); tooltip.appendText(entry.note); }
        item.tooltip = tooltip;
        item.iconPath = entryIcon(kind.id, entry);
        if (entry.path && entry.line) {
            item.command = {
                command: 'vscode.open',
                title: 'Go to the breakpoint',
                arguments: [
                    vscode.Uri.file(entry.path),
                    { selection: new vscode.Range(entry.line - 1, 0, entry.line - 1, 0) },
                ],
            };
        }
        return item;
    }
}

const KIND_ICONS: Record<string, string> = {
    source: 'debug-breakpoint',
    directive: 'symbol-keyword',
    data: 'debug-breakpoint-data',
    watch: 'eye',
    entry: 'debug-start',
    [EMULATOR_KIND_ID]: 'vm',
};

const STATUS_TEXT: Record<Entry['status'], string> = {
    armed: 'Armed as asked.',
    partial: 'Armed, but part of it is not applied.',
    inactive: 'Not armed.',
};

function entryIcon(kind: string, entry: Entry): vscode.ThemeIcon {
    if (entry.status === 'inactive') { return new vscode.ThemeIcon('debug-breakpoint-unverified'); }
    if (entry.status === 'partial') {
        return new vscode.ThemeIcon('warning', new vscode.ThemeColor('list.warningForeground'));
    }
    if (entry.log) { return new vscode.ThemeIcon('debug-breakpoint-log'); }
    if (entry.conditional) { return new vscode.ThemeIcon('debug-breakpoint-conditional'); }
    return new vscode.ThemeIcon(KIND_ICONS[kind] ?? 'debug-breakpoint');
}

const OWNERS: Record<string, string> = {
    source: 'a source breakpoint',
    'log point': 'a log point',
    directive: 'a BREAKPOINT directive',
    data: 'a data breakpoint',
    watch: 'a watch',
    emulator: 'not this session: the snapshot, or another tool',
};

const hex = (value: number | undefined) =>
    value === undefined ? '?' : `#${value.toString(16).toUpperCase().padStart(4, '0')}`;

/** What the emulator holds, as a kind of its own. */
function emulatorKind(list: EmulatorList): Kind {
    const entries: Entry[] = [];
    for (const address of list.addresses ?? []) {
        entries.push({
            label: address,
            detail: 'execution, on this RAM bank/ROM location',
            status: 'armed',
            note: 'An address breakpoint: a source breakpoint, a plain directive or a step over in flight.',
        });
    }
    for (const brk of list.advanced ?? []) {
        const parts = [`${brk.kind} ${hex(brk.addr)}`];
        if (brk.kind !== 'exec' && brk.access) { parts.push(brk.access); }
        if (brk.run && brk.run !== 'stop') { parts.push(brk.run); }
        if (brk.condition) { parts.push(`if ${brk.condition}`); }
        if (brk.after) { parts.push(`after ${brk.after}`); }
        parts.push(`${brk.hits ?? 0} hit${brk.hits === 1 ? '' : 's'}`);
        entries.push({
            label: brk.label,
            detail: parts.join(', '),
            status: 'armed',
            note: `Armed by ${OWNERS[brk.owner] ?? brk.owner}.`,
            conditional: Boolean(brk.condition),
        });
    }
    for (const raster of list.raster ?? []) {
        entries.push({
            label: raster.line === undefined ? `every line, NOP ${raster.nop}` : `line ${raster.line}, NOP ${raster.nop}`,
            detail: 'raster: stops when the beam gets there',
            status: 'armed',
        });
    }
    return {
        id: EMULATOR_KIND_ID,
        title: 'In the emulator',
        description:
            'What bemu itself has armed right now, whoever armed it, with how many times each was hit: '
            + 'address breakpoints (resolved to a RAM bank or ROM), advanced breakpoints (execution, memory, '
            + 'I/O, with their condition and pass count) and raster breakpoints, which stop when the beam '
            + 'reaches a position of the frame.',
        entries,
    };
}

/** Register the view and keep it up to date. */
export function registerBreakpointsView(context: vscode.ExtensionContext): void {
    const view = new BreakpointsView();
    context.subscriptions.push(
        vscode.window.registerTreeDataProvider('cpclib.breakpoints', view),
        vscode.commands.registerCommand('cpclib.refreshBreakpoints', () => view.refresh()),
        vscode.debug.onDidChangeBreakpoints(() => view.refresh()),
        vscode.debug.onDidChangeActiveDebugSession(() => view.refresh()),
        vscode.debug.onDidTerminateDebugSession(() => view.refresh()),
        // A stop changes hit counts and retires the stop on entry.
        vscode.debug.onDidReceiveDebugSessionCustomEvent(event => {
            if (['cpclib/emulatorReady', 'cpclib/stoppedAt', 'cpclib/stoppedWithoutSource'].includes(event.event)) {
                view.refresh();
            }
        }),
    );
}
