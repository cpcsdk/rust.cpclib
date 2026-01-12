// ============================================================================
// HELPER UTILITIES
// ============================================================================

// Common highlight animation constants
const HIGHLIGHT_COLOR = '#fff3cd';
const HIGHLIGHT_DURATION = 2000; // ms
const HEADER_OFFSET = -80; // px

// Highlight element temporarily
function highlightElement(element, duration = HIGHLIGHT_DURATION) {
    if (!element) return;
    element.style.backgroundColor = HIGHLIGHT_COLOR;
    element.style.transition = 'background-color 0.3s';
    setTimeout(function() {
        element.style.backgroundColor = '';
    }, duration);
}

// Smooth scroll with header offset
function smoothScrollTo(element, block = 'center') {
    if (!element) return;
    element.scrollIntoView({ behavior: 'smooth', block: block });
}

// Find ancestor element by condition
function findAncestor(element, condition) {
    let current = element;
    while (current) {
        if (condition(current)) return current;
        current = current.parentElement;
    }
    return null;
}

// ============================================================================
// FILE TREE - Hierarchical display of source files
// ============================================================================

// Build file tree from flat file list
function buildFileTree(files) {
    const root = {};
    
    files.forEach(file => {
        const parts = file.split('/');
        let current = root;
        
        parts.forEach((part, index) => {
            if (!current[part]) {
                current[part] = {
                    name: part,
                    fullPath: parts.slice(0, index + 1).join('/'),
                    isFile: index === parts.length - 1,
                    realFile: index === parts.length - 1 ? file : null,  // Store original file path for leaf nodes
                    children: {}
                };
            }
            current = current[part].children;
        });
    });
    
    return root;
}

// Render file tree HTML
function renderFileTree(node, isRoot = false) {
    let html = '';
    const entries = Object.values(node);
    
    if (entries.length === 0) return html;
    
    if (!isRoot) html += '<ul class="file-tree-list">';
    else html = '<ul class="file-tree-list">';
    
    entries.sort((a, b) => {
        // Folders first, then files
        if (a.isFile !== b.isFile) return a.isFile ? 1 : -1;
        return a.name.localeCompare(b.name);
    }).forEach(entry => {
        if (entry.isFile) {
            html += `<li class="file-tree-item file-tree-file">
                <a href="#" class="file-filter-link" data-file="${entry.realFile || entry.fullPath}">
                    📄 ${entry.name}
                </a>
            </li>`;
        } else {
            const hasChildren = Object.keys(entry.children).length > 0;
            html += `<li class="file-tree-item file-tree-folder">
                <details open>
                    <summary class="file-tree-folder-name">📁 ${entry.name}</summary>
                    ${renderFileTree(entry.children, false)}
                </details>
            </li>`;
        }
    });
    
    html += '</ul>';
    return html;
}

// ============================================================================
// SOURCE CODE DISPLAY - Toggle visibility and highlighting
// ============================================================================

// Toggle source code visibility
function toggleSource(id) {
    const content = document.getElementById(id);
    const button = event.target;
    
    if (content.classList.contains('show')) {
        content.classList.remove('show');
        button.classList.remove('active');
    } else {
        content.classList.add('show');
        button.classList.add('active');
        // Mark code as ready - syntax highlighting is done server-side in Rust
        const codeBlock = content.querySelector('code');
        if (codeBlock && !codeBlock.dataset.highlighted) {
            codeBlock.dataset.highlighted = 'true';
        }
    }
}

// ============================================================================
// FILE FILTERING - Show/hide items by source file
// ============================================================================

// File filtering functionality
function filterByFile(fileName) {
    const items = document.querySelectorAll('.item');
    const sections = document.querySelectorAll('.section');
    const fileLinks = document.querySelectorAll('.file-filter-link');
    const sidebarItems = document.querySelectorAll('.sidebar-item');
    const sidebarSections = document.querySelectorAll('.sidebar-section');
    const indexItems = document.querySelectorAll('.index-item');
    const indexLetters = document.querySelectorAll('.index-letter');
    const allFilesLink = document.getElementById('all-files-link');
    
    // Update active link
    fileLinks.forEach(link => {
        if (link.dataset.file === fileName) {
            link.classList.add('active');
        } else {
            link.classList.remove('active');
        }
    });
    
    // Update "All Files" link active state
    if (allFilesLink) {
        if (fileName === '') {
            allFilesLink.classList.add('active');
        } else {
            allFilesLink.classList.remove('active');
        }
    }
    
    // Scroll to file documentation if filtering by specific file
    if (fileName !== '') {
        const fileAnchorId = 'file_' + fileName.replace(/[\/\\.]/g, '_');
        const fileAnchor = document.getElementById(fileAnchorId);
        if (fileAnchor) {
            setTimeout(() => {
                fileAnchor.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }, 100);
        }
    }
    
    // Filter symbol index items
    indexItems.forEach(item => {
        const itemFile = item.dataset.sourceFile;
        // debug: show item mapping
        // console.debug('[doc] index-item', itemFile);
        
        if (fileName === '' || itemFile === fileName) {
            item.classList.remove('filtered');
        } else {
            item.classList.add('filtered');
        }
    });
    
    // Hide/show index letter headers based on visible items
    indexLetters.forEach(letterHeader => {
        const letterDiv = letterHeader.nextElementSibling; // The index-items div
        if (letterDiv && letterDiv.classList.contains('index-items')) {
            const visibleItems = letterDiv.querySelectorAll('.index-item:not(.filtered)');
            const letterId = letterHeader.id; // e.g., "index-A"
            const letter = letterId.replace('index-', '');
            const jumpLink = document.querySelector(`.jump-link[data-letter="${letter}"]`);
            
            if (visibleItems.length === 0) {
                letterHeader.classList.add('hidden');
                letterDiv.classList.add('hidden');
                if (jumpLink) jumpLink.classList.add('hidden');
            } else {
                letterHeader.classList.remove('hidden');
                letterDiv.classList.remove('hidden');
                if (jumpLink) jumpLink.classList.remove('hidden');
            }
        }
    });
    
    // Hide/show entire symbol index section if no items are visible
    const allVisibleIndexItems = document.querySelectorAll('.index-item:not(.filtered):not(.hidden)');
    const symbolIndexSection = document.getElementById('symbol-index-section');
    const symbolIndexSidebar = document.getElementById('sidebar-symbol-index');
    
    if (allVisibleIndexItems.length === 0) {
        if (symbolIndexSection) symbolIndexSection.style.display = 'none';
        if (symbolIndexSidebar) symbolIndexSidebar.style.display = 'none';
    } else {
        if (symbolIndexSection) symbolIndexSection.style.display = 'block';
        if (symbolIndexSidebar) symbolIndexSidebar.style.display = 'block';
    }
    
    // Filter sidebar items and hide empty sections
    sidebarSections.forEach(sidebarSection => {
        // Skip symbol index sidebar - it's handled above
        // Skip files sidebar - it should always be visible for navigation
        if (sidebarSection.id === 'sidebar-symbol-index' || sidebarSection.id === 'sidebar-files') {
            return;
        }
        
        let visibleSidebarCount = 0;
        const sidebarSectionItems = sidebarSection.querySelectorAll('.sidebar-item');
        
        sidebarSectionItems.forEach(item => {
            const itemFile = item.dataset.sourceFile;
            
            if (fileName === '' || itemFile === fileName) {
                item.classList.remove('filtered');
                visibleSidebarCount++;
            } else {
                item.classList.add('filtered');
            }
        });
        
        // Hide sidebar section if no visible items
        if (visibleSidebarCount === 0) {
            sidebarSection.style.display = 'none';
        } else {
            sidebarSection.style.display = 'block';
        }
    });
    
    // Filter main content items
    sections.forEach(section => {
        // Skip symbol index section - it's handled above
        if (section.id === 'symbol-index-section') {
            return;
        }
        
        let visibleCount = 0;
        const sectionItems = section.querySelectorAll('.item');
        
        sectionItems.forEach(item => {
            const itemFile = item.dataset.sourceFile;
            // debug: show main item file mapping
            // console.debug('[doc] main-item', itemFile);
            
            if (fileName === '' || itemFile === fileName) {
                item.classList.remove('filtered');
                visibleCount++;
            } else {
                item.classList.add('filtered');
            }
        });
        
        // Show/hide section based on whether it has visible items
        if (visibleCount === 0) {
            section.style.display = 'none';
        } else {
            section.style.display = 'block';
        }
    });

    // Fallback: if no main items matched this file, try finding items by ref-location
    const visibleMainItems = document.querySelectorAll('.item:not(.filtered)');
    if (fileName !== '' && visibleMainItems.length === 0) {
        const refSpans = document.querySelectorAll('.ref-location');
        const baseName = fileName.split('/').pop();
        for (const span of refSpans) {
            const txt = (span.textContent || '').trim();
            // Match patterns like "IntroHBL/src/intro_code.asm:419" or just basename
            if (txt.startsWith(fileName + ':') || (baseName && txt.startsWith(baseName + ':')) || txt.includes('/' + fileName + ':')) {
                const target = span.closest('.item') || span.closest('.index-item') || span.closest('.section');
                if (target) {
                    target.classList.remove('filtered');
                    // ensure the section is visible
                    const parentSection = target.closest('.section');
                    if (parentSection) parentSection.style.display = 'block';
                    target.scrollIntoView({ behavior: 'smooth', block: 'start' });
                    break;
                }
            }
        }
    }
}

// ============================================================================
// CLIPBOARD - Copy code to clipboard with feedback
// ============================================================================

// Copy-to-clipboard functionality
function copyToClipboard(text, button) {
    navigator.clipboard.writeText(text).then(function() {
        const originalText = button.textContent;
        button.textContent = '✓ Copied!';
        button.classList.add('success');
        
        setTimeout(function() {
            button.textContent = originalText;
            button.classList.remove('success');
        }, 2000);
    }).catch(function(err) {
        console.error('Failed to copy:', err);
        button.textContent = '✗ Failed';
        setTimeout(function() {
            button.textContent = '📋 Copy';
        }, 2000);
    });
}

// Search functionality
document.addEventListener('DOMContentLoaded', function() {
    const searchBox = document.getElementById('searchBox');
    const clearSearchBtn = document.getElementById('clearSearchBtn');
    const items = document.querySelectorAll('.item');
    const sections = document.querySelectorAll('.section');
    
    // Build file tree if files are present
    const fileTreeRoot = document.getElementById('file-tree-root');
    if (fileTreeRoot) {
        // Get all file-filter-link elements to extract file list
        const allFileLinks = document.querySelectorAll('#file-filter-list .file-filter-link[data-file]');
        const files = Array.from(allFileLinks)
            .map(link => link.dataset.file)
            .filter(file => file !== ''); // Exclude empty strings
        
        if (files.length > 0) {
            const tree = buildFileTree(files);
            fileTreeRoot.innerHTML = renderFileTree(tree, true);
            
            // Attach click handlers to all file links in the tree
            fileTreeRoot.querySelectorAll('.file-filter-link').forEach(link => {
                link.addEventListener('click', function(e) {
                    e.preventDefault();
                    filterByFile(this.dataset.file);
                });
            });
        }
    }
    
    // Attach click handler to "All Files" link
    const allFilesLink = document.getElementById('all-files-link');
    if (allFilesLink) {
        allFilesLink.addEventListener('click', function(e) {
            e.preventDefault();
            filterByFile('');
        });
    }
    
    // Function to clear search
    function clearSearch() {
        searchBox.value = '';
        performSearch();
        searchBox.focus();
    }
    
    // Update clear button visibility based on search input
    function updateClearButtonVisibility() {
        if (searchBox.value.trim() !== '') {
            clearSearchBtn.classList.add('visible');
        } else {
            clearSearchBtn.classList.remove('visible');
        }
    }
    
    // Function to perform search filtering
    function performSearch() {
        updateClearButtonVisibility();
        const searchTerm = searchBox.value.toLowerCase().trim();
        const indexItems = document.querySelectorAll('.index-item');
        const indexLetters = document.querySelectorAll('.index-letter');
        
        // Filter symbol index items by search term
        indexItems.forEach(item => {
            const codeElement = item.querySelector('code');
            if (!codeElement) return;
            
            const symbolName = codeElement.textContent.toLowerCase();
            
            if (searchTerm === '' || symbolName.includes(searchTerm)) {
                item.classList.remove('hidden');
            } else {
                item.classList.add('hidden');
            }
        });
        
        // Hide/show index letter headers based on visible items
        indexLetters.forEach(letterHeader => {
            const letterDiv = letterHeader.nextElementSibling; // The index-items div
            if (letterDiv && letterDiv.classList.contains('index-items')) {
                const visibleItems = letterDiv.querySelectorAll('.index-item:not(.filtered):not(.hidden)');
                const letterId = letterHeader.id; // e.g., "index-A"
                const letter = letterId.replace('index-', '');
                const jumpLink = document.querySelector(`.jump-link[data-letter="${letter}"]`);
                
                if (visibleItems.length === 0) {
                    letterHeader.classList.add('hidden');
                    letterDiv.classList.add('hidden');
                    if (jumpLink) jumpLink.classList.add('hidden');
                } else {
                    letterHeader.classList.remove('hidden');
                    letterDiv.classList.remove('hidden');
                    if (jumpLink) jumpLink.classList.remove('hidden');
                }
            }
        });
        
        // Hide/show entire symbol index section if no items are visible
        const allVisibleIndexItems = document.querySelectorAll('.index-item:not(.filtered):not(.hidden)');
        const symbolIndexSection = document.getElementById('symbol-index-section');
        const symbolIndexSidebar = document.getElementById('sidebar-symbol-index');
        
        if (allVisibleIndexItems.length === 0) {
            if (symbolIndexSection) symbolIndexSection.style.display = 'none';
            if (symbolIndexSidebar) symbolIndexSidebar.style.display = 'none';
        } else {
            if (symbolIndexSection) symbolIndexSection.style.display = 'block';
            if (symbolIndexSidebar) symbolIndexSidebar.style.display = 'block';
        }
        
        sections.forEach(section => {
            // Skip symbol index section - it's handled above
            if (section.id === 'symbol-index-section') {
                return;
            }
            
            let visibleCount = 0;
            const sectionItems = section.querySelectorAll('.item');
            
            sectionItems.forEach(item => {
                // Skip if already filtered by file
                if (item.classList.contains('filtered')) {
                    return;
                }
                
                const title = item.querySelector('h4').textContent.toLowerCase();
                const content = item.textContent.toLowerCase();
                
                if (searchTerm === '' || title.includes(searchTerm) || content.includes(searchTerm)) {
                    item.classList.remove('hidden');
                    visibleCount++;
                } else {
                    item.classList.add('hidden');
                }
            });
            
            // Show/hide section based on whether it has visible items
            if (visibleCount === 0 && searchTerm !== '') {
                section.style.display = 'none';
            } else {
                section.style.display = 'block';
            }
        });
        
        // Check if any items are visible
        const visibleItems = document.querySelectorAll('.item:not(.hidden):not(.filtered)');
        const noResultsMsg = document.querySelector('.no-results');
        
        if (visibleItems.length === 0 && searchTerm !== '') {
            if (!noResultsMsg) {
                const msg = document.createElement('div');
                msg.className = 'no-results visible';
                msg.textContent = 'No results found for "' + searchTerm + '"';
                document.querySelector('.main-content').appendChild(msg);
            } else {
                noResultsMsg.textContent = 'No results found for "' + searchTerm + '"';
                noResultsMsg.classList.add('visible');
            }
        } else if (noResultsMsg) {
            noResultsMsg.classList.remove('visible');
        }
    }
    
    // Apply search on input
    searchBox.addEventListener('input', performSearch);
    
    // Clear search button click handler
    clearSearchBtn.addEventListener('click', clearSearch);
    
    // Apply search on page load if browser restored value
    if (searchBox.value.trim() !== '') {
        performSearch();
    }
    
    // Clear search when clicking sidebar links
    document.querySelectorAll('.sidebar-item a').forEach(function(link) {
        link.addEventListener('click', function() {
            clearSearch();
        });
    });
    
    // Note: Syntax highlighting is done server-side in Rust during HTML generation.
    // Code blocks are already highlighted with <span class="hljs-*"> tags.
    // We only need to mark them to track their state.
    document.querySelectorAll('pre code').forEach(function(codeBlock) {
        if (!codeBlock.dataset.highlighted) {
            codeBlock.dataset.highlighted = 'true';
        }
    });
    
    // Add copy buttons to all code blocks
    document.querySelectorAll('pre code').forEach(function(codeBlock) {
        const pre = codeBlock.parentElement;
        const button = document.createElement('button');
        button.className = 'copy-btn';
        button.textContent = '📋 Copy';
        button.onclick = function() {
            copyToClipboard(codeBlock.textContent, button);
        };
        pre.appendChild(button);
    });
    
    // Setup lazy-loading for compressed code blocks
    setupLazyCodeLoading();
    
    // Setup references toggle with memory management
    setupReferencesToggle();

});

// ============================================================================
// REFERENCES TOGGLE - Show/hide references with memory management
// ============================================================================

// Handle references toggle to show/hide cross-references and free memory when collapsed
function setupReferencesToggle() {
    document.querySelectorAll('.cross-references details').forEach(function(details) {
        const summary = details.querySelector('summary.references-toggle');
        if (!summary) return;
        
        // Extract the count from the original text (e.g., "📋 Show References (5)")
        const originalText = summary.textContent;
        const countMatch = originalText.match(/\((\d+)\)/);
        const count = countMatch ? countMatch[1] : '';
        const showText = originalText;
        const hideText = `📋 Hide References${count ? ' (' + count + ')' : ''}`;
        
        details.addEventListener('toggle', function() {
            if (details.open) {
                // Opening - change text to "Hide References"
                summary.textContent = hideText;
            } else {
                // Closing - change text back to "Show References" and remove content from DOM
                summary.textContent = showText;
                
                // Remove the reference list from DOM to free memory
                const referenceList = details.querySelector('.reference-list');
                if (referenceList && !details.dataset.originalContent) {
                    // Store original content before removing (for re-insertion)
                    details.dataset.originalContent = referenceList.outerHTML;
                    referenceList.remove();
                } else if (referenceList && details.dataset.originalContent) {
                    // Already stored, just remove
                    referenceList.remove();
                }
            }
        }, { once: false });
        
        // Re-insert content when opened again
        details.addEventListener('toggle', function() {
            if (details.open && details.dataset.originalContent && !details.querySelector('.reference-list')) {
                // Re-insert the stored content
                const tempDiv = document.createElement('div');
                tempDiv.innerHTML = details.dataset.originalContent;
                const restoredList = tempDiv.firstChild;
                details.appendChild(restoredList);
            }
        }, { once: false });
    });
}

// ============================================================================
// LAZY LOADING - Decompress and display code on demand
// ============================================================================

// Lazy loading for compressed code blocks using native DecompressionStream API
// Code is stored in a centralized COMPRESSED_CODE map and referenced by ID (reduces file size by ~33%)
function setupLazyCodeLoading() {
    document.querySelectorAll('details.lazy-code').forEach(function(details) {
        const summary = details.querySelector('summary');
        const originalText = summary ? summary.textContent : 'Show Source';
        
        // Only decompress when the details element is opened
        details.addEventListener('toggle', function() {
            if (details.open && !details.dataset.loaded) {
                // Update button text to "Hide Source"
                if (summary) {
                    summary.textContent = 'Hide Source';
                }
                
                const placeholder = details.querySelector('.code-placeholder');
                const codeId = placeholder ? placeholder.dataset.codeId : null;
                
                if (codeId && COMPRESSED_CODE[codeId]) {
                    // Decompress using native DecompressionStream API (supported in all modern browsers)
                    decompressCode(COMPRESSED_CODE[codeId])
                        .then(function(decompressed) {
                            // Replace placeholder with actual content
                            placeholder.outerHTML = decompressed;
                            details.dataset.loaded = 'true';
                            
                            // Add copy button to the newly inserted code block
                            const codeBlock = details.querySelector('pre code');
                            if (codeBlock && !codeBlock.parentElement.querySelector('.copy-btn')) {
                                const pre = codeBlock.parentElement;
                                const button = document.createElement('button');
                                button.className = 'copy-btn';
                                button.textContent = '📋 Copy';
                                button.onclick = function() {
                                    copyToClipboard(codeBlock.textContent, button);
                                };
                                pre.appendChild(button);
                            }
                        })
                        .catch(function(e) {
                            console.error('Failed to decompress code:', e);
                            placeholder.innerHTML = '<pre><code>Error loading code content. Please use a modern browser.</code></pre>';
                        });
                } else {
                    console.warn('No compressed data found for code ID:', codeId);
                }
            } else if (!details.open && details.dataset.loaded === 'true') {
                // Collapsed: remove content from DOM to free memory and reset button text
                if (summary) {
                    summary.textContent = originalText;
                }
                
                // Find and remove all code content (pre, code elements), leaving only the placeholder structure
                const codeContainer = details.querySelector('pre');
                if (codeContainer) {
                    // Create fresh placeholder
                    const placeholder = details.querySelector('.code-placeholder');
                    const codeId = placeholder ? placeholder.dataset.codeId : details.dataset.codeId;
                    
                    // Remove the entire code content
                    codeContainer.remove();
                    
                    // Re-insert placeholder for next open
                    const newPlaceholder = document.createElement('div');
                    newPlaceholder.className = 'code-placeholder';
                    newPlaceholder.dataset.codeId = codeId;
                    newPlaceholder.textContent = 'Loading...';
                    details.appendChild(newPlaceholder);
                    
                    // Mark as unloaded so it can be loaded again
                    details.dataset.loaded = 'false';
                }
            } else if (details.open && details.dataset.loaded === 'true') {
                // Already loaded and opening again - just update button text
                if (summary) {
                    summary.textContent = 'Hide Source';
                }
            }
        }, { once: false });
    });
}

// ASCII85 decoder - converts ASCII85 encoded string to Uint8Array
// ASCII85 is more efficient than base64 (80% vs 75% efficiency, ~6-7% smaller)
function decodeAscii85(str) {
    const bytes = [];
    let tuple = 0;
    let count = 0;
    
    for (let i = 0; i < str.length; i++) {
        const char = str.charAt(i);
        
        // Skip whitespace
        if (char === ' ' || char === '\t' || char === '\r' || char === '\n') {
            continue;
        }
        
        // Handle 'z' special case (represents four null bytes)
        if (char === 'z') {
            if (count !== 0) {
                throw new Error('Invalid ASCII85 encoding: z in middle of tuple');
            }
            bytes.push(0, 0, 0, 0);
            continue;
        }
        
        // Decode character (valid range: ! to u, ASCII 33-117)
        const value = char.charCodeAt(0) - 33;
        if (value < 0 || value > 84) {
            throw new Error('Invalid ASCII85 character: ' + char);
        }
        
        tuple = tuple * 85 + value;
        count++;
        
        if (count === 5) {
            // Convert tuple to 4 bytes (big-endian)
            bytes.push(
                (tuple >>> 24) & 0xFF,
                (tuple >>> 16) & 0xFF,
                (tuple >>> 8) & 0xFF,
                tuple & 0xFF
            );
            tuple = 0;
            count = 0;
        }
    }
    
    // Handle remaining bytes
    if (count > 0) {
        // Pad with 'u' characters (ASCII85 for 84)
        for (let i = count; i < 5; i++) {
            tuple = tuple * 85 + 84;
        }
        
        // Output only the valid bytes
        for (let i = 0; i < count - 1; i++) {
            bytes.push((tuple >>> (24 - i * 8)) & 0xFF);
        }
    }
    
    return new Uint8Array(bytes);
}

// Native browser DecompressionStream API for gzip decompression
async function decompressCode(ascii85Data) {
    // Decode ASCII85 to binary
    const bytes = decodeAscii85(ascii85Data);
    
    // Create a ReadableStream from the compressed data
    const stream = new ReadableStream({
        start(controller) {
            controller.enqueue(bytes);
            controller.close();
        }
    });
    
    // Decompress using native DecompressionStream
    const decompressedStream = stream.pipeThrough(new DecompressionStream('gzip'));
    
    // Read the decompressed data
    const reader = decompressedStream.getReader();
    const chunks = [];
    
    while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
    }
    
    // Combine chunks and decode as UTF-8 text
    const totalLength = chunks.reduce((acc, chunk) => acc + chunk.length, 0);
    const combined = new Uint8Array(totalLength);
    let offset = 0;
    for (const chunk of chunks) {
        combined.set(chunk, offset);
        offset += chunk.length;
    }
    
    return new TextDecoder().decode(combined);
}

// ============================================================================
// NAVIGATION - Scroll to source files and specific line numbers
// ============================================================================

// Navigate to a source file section and optionally a specific line
function navigateToSourceLine(filename, lineNumber) {
    const anchorId = 'source_' + filename.replace(/[/.\\]/g, '_');
    const section = document.getElementById(anchorId);
    
    if (!section) return;
    
    const details = section.parentElement.querySelector('details.lazy-code, details.macro-source-details');
    
    if (details) {
        if (!details.open) {
            details.open = true;
        }
        
        if (!details.dataset.loaded) {
            waitForCodeToLoad(details, function() {
                scrollToSection(section, lineNumber);
            });
        } else {
            scrollToSection(section, lineNumber);
        }
    } else {
        scrollToSection(section, lineNumber);
    }
}

// Wait for lazy-loaded code to be decompressed and inserted into DOM
function waitForCodeToLoad(details, callback) {
    const POLL_INTERVAL = 50; // ms
    const TIMEOUT = 5000; // ms
    const DOM_SETTLE_DELAY = 100; // ms - wait for browser to insert elements
    
    const checkLoaded = setInterval(function() {
        if (details.dataset.loaded === 'true') {
            clearInterval(checkLoaded);
            // Give browser time to insert DOM elements after decompression
            setTimeout(callback, DOM_SETTLE_DELAY);
        }
    }, POLL_INTERVAL);
    
    // Fallback timeout
    setTimeout(function() {
        clearInterval(checkLoaded);
        callback();
    }, TIMEOUT);
}

// Scroll to a section, optionally targeting a specific line number
function scrollToSection(section, lineNumber) {
    if (lineNumber && scrollToLine(section, lineNumber)) {
        return;
    }
    
    // Fallback: scroll to section header
    section.scrollIntoView({ behavior: 'smooth', block: 'start' });
}

// Scroll to and highlight a specific line number within a section
function scrollToLine(section, lineNumber) {
    const container = section.parentElement;
    if (!container) return false;
    
    const targetLine = container.querySelector('.line-number[id="L' + lineNumber + '"]');
    if (!targetLine) return false;
    
    // Smooth scroll with offset
    const y = targetLine.getBoundingClientRect().top + window.pageYOffset + HEADER_OFFSET;
    window.scrollTo({ top: y, behavior: 'smooth' });
    
    // Highlight the line
    highlightElement(targetLine.parentElement);
    return true;
}

// Helper function to navigate to a symbol element
function navigateToSymbol(targetElement) {
    // Open any collapsed parent details
    const parent = findAncestor(targetElement, el => el.tagName === 'DETAILS' && !el.open);
    if (parent) {
        parent.open = true;
        
        // Handle lazy-loaded code
        if (parent.classList.contains('lazy-code') || parent.classList.contains('macro-source-details')) {
            waitForCodeToLoad(parent, function() {
                smoothScrollTo(targetElement);
                highlightElement(targetElement);
            });
            return;
        }
    }
    
    // Scroll and highlight
    smoothScrollTo(targetElement);
    highlightElement(targetElement);
}

// Set up click handlers for ref-location elements (e.g., "file.asm:123")
document.addEventListener('DOMContentLoaded', function() {
    document.addEventListener('click', function(e) {
        if (e.target.classList.contains('ref-location')) {
            e.preventDefault();
            const text = e.target.textContent.trim();
            const parts = text.split(':');
            
            if (parts.length >= 2) {
                const filename = parts[0];
                const lineNumber = parseInt(parts[1], 10);
                navigateToSourceLine(filename, lineNumber);
            }
        }
        
        // Handle symbol-link clicks - navigate to the symbol's definition
        if (e.target.classList.contains('symbol-link')) {
            e.preventDefault();
            const targetId = e.target.getAttribute('href')?.substring(1);
            
            if (targetId) {
                const targetElement = document.getElementById(targetId);
                if (!targetElement) return;
                
                // Find which file this symbol belongs to
                const symbolItem = findAncestor(targetElement, el => 
                    el.classList.contains('item') && el.dataset.sourceFile
                );
                
                if (symbolItem) {
                    const symbolFile = symbolItem.dataset.sourceFile;
                    const currentFilter = document.querySelector('.file-filter-link.active')?.dataset.file || '';
                    
                    // Switch files if needed
                    if (currentFilter && currentFilter !== symbolFile) {
                        filterByFile(symbolFile);
                        setTimeout(function() {
                            navigateToSymbol(targetElement);
                        }, 150);
                        return;
                    }
                }
                
                navigateToSymbol(targetElement);
            }
        }
    });
    
    // ========================================================================
    // DARK MODE - Theme toggle with localStorage persistence
    // ========================================================================
    
    const themeToggle = document.getElementById('themeToggle');
    const themeIcon = themeToggle ? themeToggle.querySelector('.theme-icon') : null;
    
    // Load saved theme or default to light
    const savedTheme = localStorage.getItem('theme') || 'light';
    if (savedTheme === 'dark') {
        document.body.classList.add('dark-mode');
        if (themeIcon) themeIcon.textContent = '☀️';
    }
    
    // Toggle theme on click
    if (themeToggle) {
        themeToggle.addEventListener('click', function() {
            document.body.classList.toggle('dark-mode');
            const isDark = document.body.classList.contains('dark-mode');
            
            if (themeIcon) {
                themeIcon.textContent = isDark ? '☀️' : '🌙';
            }
            
            localStorage.setItem('theme', isDark ? 'dark' : 'light');
        });
    }
    
    // ========================================================================
    // SEARCH HIGHLIGHTING - Highlight matched text in search results
    // ========================================================================
    
    const searchBox = document.getElementById('searchBox');
    const originalSearch = window.searchDocumentation;
    
    // Helper to remove all highlights
    function removeHighlights() {
        document.querySelectorAll('mark.search-highlight').forEach(function(mark) {
            const parent = mark.parentNode;
            parent.replaceChild(document.createTextNode(mark.textContent), mark);
            parent.normalize(); // Merge adjacent text nodes
        });
    }
    
    // Helper to highlight text in an element
    function highlightText(element, searchTerm) {
        if (!searchTerm || searchTerm.length < 2) return;
        
        const walker = document.createTreeWalker(
            element,
            NodeFilter.SHOW_TEXT,
            {
                acceptNode: function(node) {
                    // Skip if parent is already a mark or is a script/style tag
                    if (node.parentElement.tagName === 'MARK' || 
                        node.parentElement.tagName === 'SCRIPT' ||
                        node.parentElement.tagName === 'STYLE') {
                        return NodeFilter.FILTER_REJECT;
                    }
                    return NodeFilter.FILTER_ACCEPT;
                }
            }
        );
        
        const nodesToHighlight = [];
        let node;
        while (node = walker.nextNode()) {
            nodesToHighlight.push(node);
        }
        
        const regex = new RegExp('(' + searchTerm.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + ')', 'gi');
        
        nodesToHighlight.forEach(function(textNode) {
            const text = textNode.textContent;
            if (regex.test(text)) {
                const fragment = document.createDocumentFragment();
                let lastIndex = 0;
                
                text.replace(regex, function(match, p1, offset) {
                    // Add text before match
                    if (offset > lastIndex) {
                        fragment.appendChild(document.createTextNode(text.substring(lastIndex, offset)));
                    }
                    
                    // Add highlighted match
                    const mark = document.createElement('mark');
                    mark.className = 'search-highlight';
                    mark.textContent = match;
                    fragment.appendChild(mark);
                    
                    lastIndex = offset + match.length;
                });
                
                // Add remaining text
                if (lastIndex < text.length) {
                    fragment.appendChild(document.createTextNode(text.substring(lastIndex)));
                }
                
                textNode.parentNode.replaceChild(fragment, textNode);
            }
        });
    }
    
    // Enhanced search with highlighting
    if (searchBox && originalSearch) {
        window.searchDocumentation = function() {
            removeHighlights();
            
            const searchTerm = searchBox.value.trim();
            originalSearch(); // Call original search function
            
            if (searchTerm.length >= 2) {
                // Highlight in visible items
                document.querySelectorAll('.item:not(.hidden), .index-item:not(.filtered)').forEach(function(item) {
                    highlightText(item, searchTerm);
                });
            }
        };
    }
    
    // ========================================================================
    // KEYBOARD SHORTCUTS - Global keyboard navigation
    // ========================================================================
    
    const keyboardHelp = document.getElementById('keyboardHelp');
    let helpTimeout;
    
    // Show help tooltip temporarily
    function showKeyboardHelp() {
        if (!keyboardHelp) return;
        clearTimeout(helpTimeout);
        keyboardHelp.classList.add('visible');
        helpTimeout = setTimeout(function() {
            keyboardHelp.classList.remove('visible');
        }, 3000);
    }
    
    // Global keyboard handler
    document.addEventListener('keydown', function(e) {
        // / - Focus search
        if (e.key === '/' && !e.ctrlKey && !e.metaKey && !e.altKey) {
            if (document.activeElement.tagName !== 'INPUT' && 
                document.activeElement.tagName !== 'TEXTAREA') {
                e.preventDefault();
                if (searchBox) {
                    searchBox.focus();
                    searchBox.select();
                    showKeyboardHelp();
                }
            }
        }
        
        // Escape - Clear search
        if (e.key === 'Escape') {
            if (searchBox && document.activeElement === searchBox) {
                searchBox.value = '';
                removeHighlights();
                if (window.searchDocumentation) {
                    window.searchDocumentation();
                }
                searchBox.blur();
            }
        }
        
        // Ctrl+K or Cmd+K - Focus search (like many modern apps)
        if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
            e.preventDefault();
            if (searchBox) {
                searchBox.focus();
                searchBox.select();
                showKeyboardHelp();
            }
        }
        
        // Ctrl+Shift+D - Toggle dark mode
        if (e.ctrlKey && e.shiftKey && e.key === 'D') {
            e.preventDefault();
            if (themeToggle) {
                themeToggle.click();
                showKeyboardHelp();
            }
        }
        
        // ? - Show keyboard shortcuts
        if (e.key === '?' && !e.ctrlKey && !e.metaKey && !e.altKey) {
            if (document.activeElement.tagName !== 'INPUT' && 
                document.activeElement.tagName !== 'TEXTAREA') {
                e.preventDefault();
                showKeyboardHelp();
            }
        }
    });
});

