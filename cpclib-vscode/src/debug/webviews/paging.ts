import { escapeHtml } from '../../shared/html';

/**
 * The paging form the memory and screen panels share: how their bytes are read - the CPU's
 * paging now, or another one, part by part (`-mv`/`-sv`'s `[paging]` argument, see
 * `cpclib_bemu_api::Paging`): read or write, the RAM's configuration and page, the lower ROM
 * (on a CPC+ its cartridge page and block), the upper ROM and its number, the ASIC's registers.
 * Every part starts "as now"; the text the parts make is shown, and can be typed instead.
 *
 * The form's script is source text ({@link PAGING_FORM_SCRIPT}), since a webview cannot import:
 * it parses a paging into the fields and makes one from them, the grammar the adapter checks.
 */

/** What the form shows for a view: its paging, else its RAM configuration, else nothing. */
export function pagingValue(dump: { paging?: string | null; config?: number | null }): string {
    if (dump.paging) { return dump.paging === 'cpu' ? '' : dump.paging; }
    return typeof dump.config === 'number' ? `C${dump.config}` : '';
}

const triState = (id: string, label: string, on: string, off: string, title: string) =>
    `<label title="${escapeHtml(title)}">${label} <select id="${id}">` +
    `<option value="">as now</option><option value="on">${on}</option><option value="off">${off}</option>` +
    `</select></label>`;

/** The form, its fields' ids prefixed with `prefix`. */
export function pagingFormHtml(prefix: string, dump: { paging?: string | null; config?: number | null }): string {
    const configs = ['<option value="">as now</option>']
        .concat([0, 1, 2, 3, 4, 5, 6, 7].map(n => `<option value="${n}">C${n} (&amp;C${n})</option>`))
        .join('');
    return `<fieldset class="paging" id="${prefix}-form">
  <legend>Paging</legend>
  <label title="What a read gives (the ROMs where they are paged), or what a write reaches (the RAM, under the ROMs too)">
    <select id="${prefix}-access"><option value="">Read</option><option value="W">Write</option></select></label>
  <label title="The RAM configuration: which bank each 16K block shows">RAM <select id="${prefix}-config">${configs}</select></label>
  <label title="The extension's 64K page the configuration's banks 4-7 come from">page
    <input id="${prefix}-page" type="number" min="0" max="63" placeholder="as now" size="4"></label>
  ${triState(`${prefix}-lower`, 'Lower ROM', 'paged', 'not paged', 'The firmware at &0000 (on a CPC+: a cartridge page)')}
  <label class="plus" title="CPC+: the cartridge page the lower ROM is">cart. page
    <input id="${prefix}-lowerpage" type="number" min="0" max="7" placeholder="as now" size="3"></label>
  <label class="plus" title="CPC+: where the lower ROM is">at <select id="${prefix}-lowerblock">
    <option value="">as now</option><option value="0">&amp;0000</option><option value="1">&amp;4000</option><option value="2">&amp;8000</option></select></label>
  ${triState(`${prefix}-upper`, 'Upper ROM', 'paged', 'not paged', 'The upper ROM at &C000')}
  <label title="The upper ROM's number: 0 BASIC, 7 AMSDOS...; on a CPC+ 128-159 are the cartridge's pages 0-31">number
    <input id="${prefix}-uppernumber" type="number" min="0" max="255" placeholder="as now" size="4"></label>
  ${triState(`${prefix}-asic`, 'ASIC registers', 'at &amp;4000', 'not paged', 'CPC+: the ASIC\'s registers at &4000-&7FFF')}
  <label title="The paging the parts make (C0-C7 or &amp;C0-&amp;FF, Pn, L/-L, LPn, LBn, U/-U, Un, R/-R, W); type one instead if you like">=
    <input id="${prefix}-text" size="16" placeholder="CPU's"></label>
  <button type="button" id="${prefix}-reset" title="Back to the CPU's paging now">CPU's</button>
</fieldset>
<script type="application/json" id="${prefix}-initial">${JSON.stringify(pagingValue(dump)).replace(/</g, '\\u003c')}</script>`;
}

/** The form's style. */
export const PAGING_FORM_STYLE = `
  fieldset.paging { border: 1px solid var(--vscode-panel-border, #444); padding: 4px 8px; margin: 0 0 8px; }
  fieldset.paging legend { padding: 0 4px; }
  fieldset.paging label { margin-right: 10px; white-space: nowrap; display: inline-block; margin-bottom: 2px; }
  fieldset.paging select, fieldset.paging input { font-family: inherit; }
`;

/**
 * The form's behaviour, as script text: `pagingForm(prefix, onChange)` wires the form up and
 * returns `{ set(text), argument() }` - show a paging, and the console argument the form makes
 * (`_` for the CPU's). `onChange()` is called when a person changes it.
 */
export const PAGING_FORM_SCRIPT = `
  function pagingParse(text) {
    const parts = { access: '', config: '', page: '', lower: '', lowerpage: '', lowerblock: '',
                    upper: '', uppernumber: '', asic: '' };
    text = (text || '').replace(/\\s+/g, '');
    if (text === '' || text === '_' || text.toLowerCase() === 'cpu') { return parts; }
    let m = /^([0-7]):(\\d+)$/.exec(text);
    if (m) { parts.config = m[1]; parts.page = m[2]; return parts; }
    if (/^[0-7]$/.test(text)) { parts.config = text; return parts; }
    for (const raw of text.split(',')) {
      const part = raw.toUpperCase();
      if (part === '') { continue; }
      if (part === 'W') { parts.access = 'W'; }
      else if (part === 'L') { parts.lower = 'on'; }
      else if (part === '-L') { parts.lower = 'off'; }
      else if (part === 'U') { parts.upper = 'on'; }
      else if (part === '-U') { parts.upper = 'off'; }
      else if (part === 'R') { parts.asic = 'on'; }
      else if (part === '-R') { parts.asic = 'off'; }
      else if ((m = /^(?:&|0X)([0-9A-F]{2})$/.exec(part)) && parseInt(m[1], 16) >= 0xC0) {
        const value = parseInt(m[1], 16);
        parts.config = String(value & 7); parts.page = String((value >> 3) & 7);
      }
      else if ((m = /^LP(\\d+)$/.exec(part))) { parts.lowerpage = m[1]; }
      else if ((m = /^LB([0-2])$/.exec(part))) { parts.lowerblock = m[1]; }
      else if ((m = /^C([0-7])$/.exec(part))) { parts.config = m[1]; }
      else if ((m = /^P(\\d+)$/.exec(part))) { parts.page = m[1]; }
      else if ((m = /^U(\\d+)$/.exec(part))) { parts.upper = 'on'; parts.uppernumber = m[1]; }
      else { return null; }
    }
    return parts;
  }

  function pagingMake(parts) {
    const out = [];
    if (parts.config !== '') { out.push('C' + parts.config); }
    if (parts.page !== '') { out.push('P' + parts.page); }
    if (parts.lower === 'on') { out.push('L'); }
    if (parts.lower === 'off') { out.push('-L'); }
    if (parts.lowerpage !== '') { out.push('LP' + parts.lowerpage); }
    if (parts.lowerblock !== '') { out.push('LB' + parts.lowerblock); }
    if (parts.upper === 'off') { out.push('-U'); }
    else if (parts.uppernumber !== '') { out.push('U' + parts.uppernumber); }
    else if (parts.upper === 'on') { out.push('U'); }
    if (parts.asic === 'on') { out.push('R'); }
    if (parts.asic === 'off') { out.push('-R'); }
    if (parts.access === 'W') { out.push('W'); }
    return out.join(',');
  }

  function pagingForm(prefix, onChange) {
    const names = ['access', 'config', 'page', 'lower', 'lowerpage', 'lowerblock', 'upper', 'uppernumber', 'asic'];
    const field = name => document.getElementById(prefix + '-' + name);
    const text = field('text');
    const form = field('form');
    function read() {
      const parts = {};
      for (const name of names) { parts[name] = field(name).value.trim(); }
      return parts;
    }
    function show(parts) {
      for (const name of names) { field(name).value = parts[name]; }
    }
    function set(value) {
      // a person mid-edit keeps what they are typing (a field of the form focused, not its button)
      const focused = document.activeElement;
      if (focused && form.contains(focused) && (focused.tagName === 'INPUT' || focused.tagName === 'SELECT')) { return; }
      const parts = pagingParse(value);
      if (parts) { show(parts); }
      text.value = value || '';
      text.style.outline = '';
    }
    for (const name of names) {
      field(name).addEventListener('change', () => {
        text.value = pagingMake(read());
        text.style.outline = '';
        onChange();
      });
    }
    text.addEventListener('change', () => {
      const parts = pagingParse(text.value);
      // not a paging: said, and left for the adapter to explain
      text.style.outline = parts ? '' : '1px solid var(--vscode-inputValidation-errorBorder, red)';
      if (parts) { show(parts); }
      onChange();
    });
    field('reset').addEventListener('click', () => {
      show(pagingParse(''));
      text.value = '';
      onChange();
    });
    set(JSON.parse(field('initial').textContent));
    return {
      set,
      argument() {
        const value = text.value.replace(/\\s+/g, '');
        return value === '' ? '_' : value;
      },
    };
  }
`;

/** The places the memory panel's "Show" field suggests: what a location names. */
export const LOCATION_SUGGESTIONS: [string, string][] = [
    ['UPPER00:0000', 'upper ROM 0 (BASIC), at &C000'],
    ['UPPER07:0000', 'upper ROM 7 (AMSDOS), at &C000'],
    ['LOWER:0000', 'the lower ROM (the firmware), at &0000'],
    ['B00:0000', 'RAM bank 0 (MB0)'],
    ['B03:0000', 'RAM bank 3 (MB3, the screen)'],
    ['B04:0000', 'RAM bank 4 (P0B0, the extension\'s first)'],
    ['CART00:0000', 'CPC+: cartridge page 0'],
    ['CART01:0000', 'CPC+: cartridge page 1'],
    ['ASIC:0000', 'CPC+: the ASIC registers, at &4000'],
];

/** The memory panel's "Show" field: an address, a label, or a location in a space. */
export function locationFieldHtml(id: string, value: string): string {
    const options = LOCATION_SUGGESTIONS
        .map(([location, label]) => `<option value="${escapeHtml(location)}">${escapeHtml(label)}</option>`)
        .join('');
    return `<label title="An address, a label or a register; or a place: Bnn:hhhh (a RAM bank), LOWER:hhhh, UPPERnn:hhhh (an upper ROM), CARTnn:hhhh (a CPC+ cartridge page), ASIC:hhhh - shown where it runs">` +
        `Show <input id="${id}" list="${id}-suggestions" size="16" value="${escapeHtml(value)}"></label>` +
        `<datalist id="${id}-suggestions">${options}</datalist>`;
}
