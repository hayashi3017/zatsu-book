"use strict";
window.search = window.search || {};
(function search(search) {
    // mdBook's default search tokenization is whitespace-based, which leaves
    // most Japanese text as one giant token and breaks substring search.
    if (!Mark || !elasticlunr) {
        return;
    }

    if (!String.prototype.startsWith) {
        String.prototype.startsWith = function (needle, pos) {
            return this.substr(!pos || pos < 0 ? 0 : +pos, needle.length) === needle;
        };
    }

    var search_wrap = document.getElementById("search-wrapper"),
        searchbar = document.getElementById("searchbar"),
        searchresults = document.getElementById("searchresults"),
        searchresults_outer = document.getElementById("searchresults-outer"),
        searchresults_header = document.getElementById("searchresults-header"),
        searchicon = document.getElementById("search-toggle"),
        content = document.getElementById("content"),

        searchindex = null,
        search_docs = null,
        doc_urls = [],
        results_options = {
            teaser_word_count: 30,
            limit_results: 30,
        },
        search_options = {
            bool: "AND",
            expand: true,
            fields: {
                title: { boost: 2 },
                body: { boost: 1 },
                breadcrumbs: { boost: 1 },
            },
        },
        mark_exclude = [],
        marker = new Mark(content),
        current_searchterm = "",
        search_config = null,
        search_build_started = false,
        URL_SEARCH_PARAM = "search",
        URL_MARK_PARAM = "highlight",
        teaser_count = 0,
        MAX_INDEX_BODY_CHARS = 800,

        SEARCH_HOTKEY_KEYCODE = 83,
        ESCAPE_KEYCODE = 27,
        DOWN_KEYCODE = 40,
        UP_KEYCODE = 38,
        SELECT_KEYCODE = 13;

    function hasFocus() {
        return searchbar === document.activeElement;
    }

    function removeChildren(elem) {
        while (elem.firstChild) {
            elem.removeChild(elem.firstChild);
        }
    }

    function normalizeText(value) {
        var text = (value === undefined || value === null ? "" : String(value)).toLowerCase();
        if (typeof text.normalize === "function") {
            return text.normalize("NFKC");
        }
        return text;
    }

    function splitSearchSegments(value) {
        return normalizeText(value)
            .split(/[\s!-/:-@\[-`{-~\u3000、。！？・「」『』（）［］【】〈〉《》〔〕…]+/u)
            .filter(function (segment) {
                return segment.length > 0;
            });
    }

    function uniqueTokens(tokens) {
        var seen = Object.create(null);
        return tokens.filter(function (token) {
            if (!token || seen[token]) {
                return false;
            }
            seen[token] = true;
            return true;
        });
    }

    function tokenizeJapaneseSegment(segment) {
        var tokens = [];
        var max_gram;
        var gram_size;
        var index;

        if (!segment) {
            return tokens;
        }

        if (segment.length === 1) {
            tokens.push(segment);
            return tokens;
        }

        max_gram = Math.min(3, segment.length);
        for (gram_size = 2; gram_size <= max_gram; gram_size += 1) {
            for (index = 0; index <= segment.length - gram_size; index += 1) {
                tokens.push(segment.slice(index, index + gram_size));
            }
        }
        tokens.push(segment);
        return tokens;
    }

    function tokenizeSearchInput(input) {
        var text = Array.isArray(input) ? input.join(" ") : input;
        var segments = splitSearchSegments(text);
        var tokens = [];

        segments.forEach(function (segment) {
            var ascii_runs;

            if (/^[a-z0-9]+$/.test(segment)) {
                tokens.push(segment);
                return;
            }

            ascii_runs = segment.match(/[a-z0-9]+/g);
            if (ascii_runs) {
                tokens = tokens.concat(ascii_runs);
            }

            tokens = tokens.concat(tokenizeJapaneseSegment(segment));
        });

        return uniqueTokens(tokens);
    }

    function highlightTerms(searchterm) {
        return uniqueTokens(splitSearchSegments(searchterm)).sort(function (left, right) {
            return right.length - left.length;
        });
    }

    function parseURL(url) {
        var anchor = document.createElement("a");
        anchor.href = url;
        return {
            source: url,
            protocol: anchor.protocol.replace(":", ""),
            host: anchor.hostname,
            port: anchor.port,
            params: (function () {
                var ret = {};
                var seg = anchor.search.replace(/^\?/, "").split("&");
                var len = seg.length;
                var i = 0;
                var item;
                for (; i < len; i += 1) {
                    if (!seg[i]) {
                        continue;
                    }
                    item = seg[i].split("=");
                    ret[item[0]] = item[1];
                }
                return ret;
            })(),
            file: (anchor.pathname.match(/\/([^/?#]+)$/i) || [null, ""])[1],
            hash: anchor.hash.replace("#", ""),
            path: anchor.pathname.replace(/^([^/])/, "/$1"),
        };
    }

    function renderURL(urlobject) {
        var url = urlobject.protocol + "://" + urlobject.host;
        var joiner = "?";
        var prop;
        if (urlobject.port !== "") {
            url += ":" + urlobject.port;
        }
        url += urlobject.path;
        for (prop in urlobject.params) {
            if (urlobject.params.hasOwnProperty(prop)) {
                url += joiner + prop + "=" + urlobject.params[prop];
                joiner = "&";
            }
        }
        if (urlobject.hash !== "") {
            url += "#" + urlobject.hash;
        }
        return url;
    }

    var escapeHTML = (function () {
        var MAP = {
            "&": "&amp;",
            "<": "&lt;",
            ">": "&gt;",
            "\"": "&#34;",
            "'": "&#39;",
        };
        return function (text) {
            return text.replace(/[&<>'"]/g, function (ch) {
                return MAP[ch];
            });
        };
    })();

    function escapeRegExp(text) {
        return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    }

    function highlightSnippet(text, terms) {
        var escaped_terms = terms
            .filter(function (term) {
                return term.length > 0;
            })
            .map(escapeRegExp);
        var pattern;
        var regex;
        var result = "";
        var last_index = 0;
        var match;

        if (escaped_terms.length === 0) {
            return escapeHTML(text);
        }

        pattern = escaped_terms.join("|");
        regex = new RegExp(pattern, "giu");

        while ((match = regex.exec(text)) !== null) {
            result += escapeHTML(text.slice(last_index, match.index));
            result += "<em>" + escapeHTML(match[0]) + "</em>";
            last_index = match.index + match[0].length;
        }

        result += escapeHTML(text.slice(last_index));
        return result;
    }

    function makeTeaser(body, terms) {
        var raw_body = body || "";
        var normalized_body = normalizeText(raw_body);
        var index = -1;
        var matched_length = 0;
        var start;
        var end;
        var snippet;

        if (!raw_body) {
            return "";
        }

        terms.some(function (term) {
            var normalized_term = normalizeText(term);
            var found_index;
            if (!normalized_term) {
                return false;
            }
            found_index = normalized_body.indexOf(normalized_term);
            if (found_index >= 0 && (index < 0 || found_index < index)) {
                index = found_index;
                matched_length = term.length;
                return true;
            }
            return false;
        });

        if (index < 0) {
            snippet = raw_body.slice(0, 160);
            if (raw_body.length > 160) {
                snippet += "...";
            }
            return highlightSnippet(snippet, terms);
        }

        start = Math.max(0, index - 40);
        end = Math.min(raw_body.length, index + matched_length + 120);
        snippet = raw_body.slice(start, end);

        if (start > 0) {
            snippet = "..." + snippet;
        }
        if (end < raw_body.length) {
            snippet += "...";
        }

        return highlightSnippet(snippet, terms);
    }

    function formatSearchMetric(count, searchterm) {
        if (count === 1) {
            return count + " search result for '" + searchterm + "':";
        }
        if (count === 0) {
            return "No search results for '" + searchterm + "'.";
        }
        return count + " search results for '" + searchterm + "':";
    }

    function formatSearchResult(result, terms) {
        var teaser = makeTeaser(result.doc.body, terms);
        var url = doc_urls[result.ref] || "";
        var query_terms = encodeURIComponent(terms.join(" ")).replace(/\'/g, "%27");
        teaser_count += 1;

        return '<a href="' + path_to_root + url + "?" + URL_MARK_PARAM + "=" + query_terms + '" aria-details="teaser_' + teaser_count + '">'
            + escapeHTML(result.doc.breadcrumbs || result.doc.title || "")
            + "</a>"
            + '<span class="teaser" id="teaser_' + teaser_count + '" aria-label="Search Result Teaser">'
            + teaser
            + "</span>";
    }

    function buildSearchDocuments(config) {
        var docs = config.index.documentStore.docs;
        var refs = Object.keys(docs).sort(function (left, right) {
            return Number(left) - Number(right);
        });
        var grouped = Object.create(null);
        var pages = [];

        refs.forEach(function (ref) {
            var doc = docs[ref];
            var url = config.doc_urls[Number(ref)];
            var page_url = url.split("#")[0];
            var page = grouped[page_url];

            if (!page) {
                page = {
                    id: String(pages.length),
                    url: page_url,
                    title: doc.title,
                    breadcrumbs: doc.breadcrumbs,
                    body_parts: [],
                };
                grouped[page_url] = page;
                pages.push(page);
            }

            if (doc.body) {
                page.body_parts.push(doc.body);
            }
            if (doc.title && doc.title !== page.title) {
                page.body_parts.push(doc.title);
            }
        });

        return pages.map(function (page) {
            return {
                id: page.id,
                url: page.url,
                title: page.title,
                breadcrumbs: page.breadcrumbs,
                // Cap indexed body size so the first search build stays tolerable.
                body: page.body_parts.join(" ").slice(0, MAX_INDEX_BODY_CHARS),
            };
        });
    }

    function ensureSearchIndex() {
        if (searchindex !== null || search_config === null) {
            return;
        }

        search_build_started = true;
        search_docs = buildSearchDocuments(search_config);
        doc_urls = [];

        elasticlunr.tokenizer = tokenizeSearchInput;
        searchindex = elasticlunr(function () {
            this.setRef("id");
            this.addField("title");
            this.addField("body");
            this.addField("breadcrumbs");
            this.saveDocument(true);
            this.pipeline.reset();
        });

        search_docs.forEach(function (doc) {
            doc_urls[doc.id] = doc.url;
            searchindex.addDoc({
                id: doc.id,
                title: doc.title,
                body: doc.body,
                breadcrumbs: doc.breadcrumbs,
            }, false);
        });
    }

    function rescoreResults(results) {
        results.forEach(function (result) {
            var url = doc_urls[result.ref] || "";
            if (url.indexOf("facts/") === 0) {
                result.score *= 2.0;
            } else if (url.indexOf("tags/") === 0) {
                result.score *= 0.8;
            } else if (url.indexOf("genres/") === 0) {
                result.score *= 0.7;
            } else {
                result.score *= 0.9;
            }
        });

        results.sort(function (left, right) {
            return right.score - left.score;
        });
        return results;
    }

    function init(config) {
        search_config = config;
        results_options = config.results_options || results_options;
        search_options = config.search_options || search_options;
        search_options.bool = "AND";
        search_options.expand = true;

        if (!search_options.fields) {
            search_options.fields = {
                title: { boost: 2 },
                body: { boost: 1 },
                breadcrumbs: { boost: 1 },
            };
        }
        if (searchbar && searchbar.placeholder === "Search this book ...") {
            searchbar.placeholder = "この本を検索";
        }

        searchicon.addEventListener("click", function () {
            searchIconClickHandler();
        }, false);
        searchbar.addEventListener("keyup", function () {
            searchbarKeyUpHandler();
        }, false);
        document.addEventListener("keydown", function (event) {
            globalKeyHandler(event);
        }, false);
        window.onpopstate = function () {
            doSearchOrMarkFromUrl();
        };
        document.addEventListener("submit", function (event) {
            event.preventDefault();
        }, false);

        doSearchOrMarkFromUrl();
    }

    function unfocusSearchbar() {
        var tmp = document.createElement("input");
        tmp.setAttribute("style", "position: absolute; opacity: 0;");
        searchicon.appendChild(tmp);
        tmp.focus();
        tmp.remove();
    }

    function clearMarks() {
        marker.unmark();
    }

    function applyMarksFromUrl() {
        var url = parseURL(window.location.href);
        var words;
        var markers;
        var index;

        if (!url.params.hasOwnProperty(URL_MARK_PARAM)) {
            return;
        }

        words = decodeURIComponent(url.params[URL_MARK_PARAM]).split(" ").filter(function (term) {
            return term.length > 0;
        });

        marker.mark(words, {
            exclude: mark_exclude,
        });

        markers = document.querySelectorAll("mark");
        function hide() {
            for (index = 0; index < markers.length; index += 1) {
                markers[index].classList.add("fade-out");
            }
            window.setTimeout(function () {
                clearMarks();
            }, 300);
        }

        for (index = 0; index < markers.length; index += 1) {
            markers[index].addEventListener("click", hide);
        }
    }

    function doSearchOrMarkFromUrl() {
        var url = parseURL(window.location.href);

        if (url.params.hasOwnProperty(URL_SEARCH_PARAM) && url.params[URL_SEARCH_PARAM] !== "") {
            showSearch(true);
            searchbar.value = decodeURIComponent((url.params[URL_SEARCH_PARAM] + "").replace(/\+/g, "%20"));
            searchbarKeyUpHandler();
        } else {
            showSearch(false);
        }

        applyMarksFromUrl();
    }

    function globalKeyHandler(event) {
        var focused;
        var next;
        var prev;

        if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || event.target.type === "textarea" || event.target.type === "text" || (!hasFocus() && /^(?:input|select|textarea)$/i.test(event.target.nodeName))) {
            return;
        }

        if (event.keyCode === ESCAPE_KEYCODE) {
            event.preventDefault();
            searchbar.classList.remove("active");
            setSearchUrlParameters("", (searchbar.value.trim() !== "") ? "push" : "replace");
            if (hasFocus()) {
                unfocusSearchbar();
            }
            showSearch(false);
            clearMarks();
        } else if (!hasFocus() && event.keyCode === SEARCH_HOTKEY_KEYCODE) {
            event.preventDefault();
            showSearch(true);
            window.scrollTo(0, 0);
            searchbar.select();
        } else if (hasFocus() && event.keyCode === DOWN_KEYCODE) {
            event.preventDefault();
            unfocusSearchbar();
            if (searchresults.firstElementChild) {
                searchresults.firstElementChild.classList.add("focus");
            }
        } else if (!hasFocus() && (event.keyCode === DOWN_KEYCODE || event.keyCode === UP_KEYCODE || event.keyCode === SELECT_KEYCODE)) {
            focused = searchresults.querySelector("li.focus");
            if (!focused) {
                return;
            }
            event.preventDefault();
            if (event.keyCode === DOWN_KEYCODE) {
                next = focused.nextElementSibling;
                if (next) {
                    focused.classList.remove("focus");
                    next.classList.add("focus");
                }
            } else if (event.keyCode === UP_KEYCODE) {
                focused.classList.remove("focus");
                prev = focused.previousElementSibling;
                if (prev) {
                    prev.classList.add("focus");
                } else {
                    searchbar.select();
                }
            } else {
                window.location.assign(focused.querySelector("a"));
            }
        }
    }

    function showSearch(yes) {
        if (yes) {
            search_wrap.classList.remove("hidden");
            searchicon.setAttribute("aria-expanded", "true");
            if (!search_build_started) {
                window.setTimeout(function () {
                    ensureSearchIndex();
                }, 0);
            }
        } else {
            var results = searchresults.children;
            var index;
            search_wrap.classList.add("hidden");
            searchicon.setAttribute("aria-expanded", "false");
            for (index = 0; index < results.length; index += 1) {
                results[index].classList.remove("focus");
            }
        }
    }

    function showResults(yes) {
        if (yes) {
            searchresults_outer.classList.remove("hidden");
        } else {
            searchresults_outer.classList.add("hidden");
        }
    }

    function searchIconClickHandler() {
        if (search_wrap.classList.contains("hidden")) {
            showSearch(true);
            window.scrollTo(0, 0);
            searchbar.select();
        } else {
            showSearch(false);
        }
    }

    function searchbarKeyUpHandler() {
        var searchterm = searchbar.value.trim();

        if (searchterm !== "") {
            searchbar.classList.add("active");
            doSearch(searchterm);
        } else {
            current_searchterm = "";
            searchbar.classList.remove("active");
            showResults(false);
            removeChildren(searchresults);
        }

        setSearchUrlParameters(searchterm, "push_if_new_search_else_replace");
        clearMarks();
    }

    function setSearchUrlParameters(searchterm, action) {
        var url = parseURL(window.location.href);
        var first_search = !url.params.hasOwnProperty(URL_SEARCH_PARAM);

        if (searchterm !== "" || action === "push_if_new_search_else_replace") {
            url.params[URL_SEARCH_PARAM] = searchterm;
            delete url.params[URL_MARK_PARAM];
            url.hash = "";
        } else {
            delete url.params[URL_MARK_PARAM];
            delete url.params[URL_SEARCH_PARAM];
        }

        if (action === "push" || (action === "push_if_new_search_else_replace" && first_search)) {
            history.pushState({}, document.title, renderURL(url));
        } else if (action === "replace" || (action === "push_if_new_search_else_replace" && !first_search)) {
            history.replaceState({}, document.title, renderURL(url));
        }
    }

    function doSearch(searchterm) {
        var results;
        var resultcount;
        var terms;
        var resultElem;
        var index;

        if (current_searchterm === searchterm) {
            return;
        }
        current_searchterm = searchterm;

        ensureSearchIndex();
        if (searchindex === null) {
            return;
        }

        results = rescoreResults(searchindex.search(searchterm, search_options));
        resultcount = Math.min(results.length, results_options.limit_results);

        searchresults_header.innerText = formatSearchMetric(resultcount, searchterm);

        terms = highlightTerms(searchterm);
        removeChildren(searchresults);
        teaser_count = 0;
        for (index = 0; index < resultcount; index += 1) {
            resultElem = document.createElement("li");
            resultElem.innerHTML = formatSearchResult(results[index], terms);
            searchresults.appendChild(resultElem);
        }

        showResults(true);
    }

    fetch(path_to_root + "searchindex.json")
        .then(function (response) {
            return response.json();
        })
        .then(function (json) {
            init(json);
        })
        .catch(function () {
            var script = document.createElement("script");
            script.src = path_to_root + "searchindex.js";
            script.onload = function () {
                init(window.search);
            };
            document.head.appendChild(script);
        });

    search.hasFocus = hasFocus;
})(window.search);
