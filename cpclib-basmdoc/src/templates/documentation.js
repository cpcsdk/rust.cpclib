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

// File filtering functionality
function filterByFile(fileName) {
    console.debug('[doc] filterByFile called with', fileName);
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

});

// Lazy loading for compressed code blocks using native DecompressionStream API
function setupLazyCodeLoading() {
    document.querySelectorAll('details.lazy-code').forEach(function(details) {
        // Only decompress when the details element is opened
        details.addEventListener('toggle', function() {
            if (details.open && !details.dataset.loaded) {
                const placeholder = details.querySelector('.code-placeholder');
                if (placeholder && placeholder.dataset.compressed) {
                    // Decompress using native DecompressionStream API (supported in all modern browsers)
                    decompressCode(placeholder.dataset.compressed)
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
                }
            }
        }, { once: false });
    });
}

// Native browser DecompressionStream API for gzip decompression
async function decompressCode(base64Data) {
    // Decode base64 to binary
    const binaryString = atob(base64Data);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
    }
    
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

