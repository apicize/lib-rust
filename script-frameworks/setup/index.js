// Stub enough of process to make browserify's util happy...
process = { env: {} }

const format = require('util').format;

const jpp = require('jsonpath-plus');

let setupOffset = 0;

function fmtMinSec(value, subZero = null) {
    if (value === 0 && subZero) {
        return subZero
    }
    const m = Math.floor(value / 60000)
    value -= m * 60000
    const s = Math.floor(value / 1000)
    value -= s * 1000
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}${(0.1).toString()[1]}${value.toString().padEnd(3, '0')}`
}


/******************************************************************
 * Global variables exposed to setup scripts
 ******************************************************************/

request = null;
variables = {};
scenario = {};
data = {};
$ = {};
outputVars = {};

jsonpath = jpp.JSONPath;

// Helper function to jsonpath-plus
function jpath(param) {
    if (typeof param === 'object') {
        return jpp.JSONPath({ ...param, json: this })
    } else if (typeof param === 'string') {
        return jpp.JSONPath({ path: param, json: this })
    } else {
        throw new Error('Argument for jp must be either a JSON path (string) or named parameters')
    }
}

Object.prototype.jp = jpath
Array.prototype.jp = jpath
String.prototype.jp = jpath
Number.prototype.jp = jpath

let logs = [];

function appendLog(type, message, ...optionalParams) {
    const timestamp = fmtMinSec(Date.now() - setupOffset)
    logs.push(`${timestamp} [${type}] ${format(message, ...optionalParams)}`)
}

console = {
    log: (msg, ...args) => appendLog('log', msg, ...args),
    info: (msg, ...args) => appendLog('info', msg, ...args),
    warn: (msg, ...args) => appendLog('warn', msg, ...args),
    error: (msg, ...args) => appendLog('error', msg, ...args),
    trace: (msg, ...args) => appendLog('trace', msg, ...args),
    debug: (msg, ...args) => appendLog('debug', msg, ...args),
};

// Request body types (matches the "type" tag of workbook request bodies)
BodyType = {
    Text: 'Text',
    JSON: 'JSON',
    XML: 'XML',
    GraphQL: 'GraphQL',
    Form: 'Form',
    Raw: 'Raw'
}

/******************************************************************
 * Base64 helpers (V8 does not include btoa/atob or TextEncoder)
 ******************************************************************/

const B64_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'
const B64_LOOKUP = Object.fromEntries([...B64_CHARS].map((c, i) => [c, i]))

function bytesToBase64(bytes) {
    let result = ''
    for (let i = 0; i < bytes.length; i += 3) {
        const b0 = bytes[i]
        const b1 = i + 1 < bytes.length ? bytes[i + 1] : 0
        const b2 = i + 2 < bytes.length ? bytes[i + 2] : 0
        result += B64_CHARS[b0 >> 2]
            + B64_CHARS[((b0 & 3) << 4) | (b1 >> 4)]
            + (i + 1 < bytes.length ? B64_CHARS[((b1 & 15) << 2) | (b2 >> 6)] : '=')
            + (i + 2 < bytes.length ? B64_CHARS[b2 & 63] : '=')
    }
    return result
}

function base64ToBytes(value) {
    // Padding and whitespace are optional
    const s = `${value}`.replace(/[\s=]/g, '')
    if (s.length % 4 === 1) {
        throw new Error('Invalid base64 string')
    }
    const bytes = new Uint8Array(Math.floor(s.length * 3 / 4))
    let buffer = 0, bits = 0, index = 0
    for (const c of s) {
        const n = B64_LOOKUP[c]
        if (n === undefined) {
            throw new Error(`Invalid base64 character "${c}"`)
        }
        buffer = (buffer << 6) | n
        bits += 6
        if (bits >= 8) {
            bits -= 8
            bytes[index++] = (buffer >> bits) & 0xFF
        }
    }
    return bytes
}

// Convert a JS string to UTF-8 bytes, or an array/typed array/ArrayBuffer to bytes
function toBytes(value) {
    if (typeof value === 'string') {
        const binary = unescape(encodeURIComponent(value))
        const bytes = new Uint8Array(binary.length)
        for (let i = 0; i < binary.length; i++) {
            bytes[i] = binary.charCodeAt(i)
        }
        return bytes
    }
    if (value instanceof ArrayBuffer) {
        return new Uint8Array(value)
    }
    if (ArrayBuffer.isView(value)) {
        return new Uint8Array(value.buffer, value.byteOffset, value.byteLength)
    }
    if (Array.isArray(value)) {
        return Uint8Array.from(value)
    }
    throw new Error('Value must be a string, array, typed array or ArrayBuffer')
}

function isBinary(value) {
    return value instanceof ArrayBuffer || ArrayBuffer.isView(value) || Array.isArray(value)
}

// Standard browser-style functions operating on "binary" (Latin-1) strings
btoa = (value) => {
    const s = `${value}`
    const bytes = new Uint8Array(s.length)
    for (let i = 0; i < s.length; i++) {
        const c = s.charCodeAt(i)
        if (c > 0xFF) {
            throw new Error('btoa: string contains characters outside of the Latin1 range, use base64.encode instead')
        }
        bytes[i] = c
    }
    return bytesToBase64(bytes)
}

atob = (value) => {
    let result = ''
    for (const b of base64ToBytes(value)) {
        result += String.fromCharCode(b)
    }
    return result
}

base64 = {
    // Encode a string (as UTF-8), array, typed array or ArrayBuffer to base64
    encode: (value) => bytesToBase64(toBytes(value)),
    // Decode base64 to a Uint8Array
    decode: (value) => base64ToBytes(value),
    // Decode base64 to a string (as UTF-8)
    decodeText: (value) => decodeURIComponent(escape(atob(value))),
}

output = (name, value) => {
    switch (typeof value) {
        case 'function':
            throw new Error('Functions cannot be output')
        case 'symbol':
            throw new Error('Symbols cannot be output')
        case 'undefined':
            delete outputVars[name]
            break
        default:
            outputVars[name] = value
            break
    }
}

/******************************************************************
 * Request helpers
 ******************************************************************/

function toStr(value) {
    return (value === undefined || value === null) ? '' : `${value}`
}

// Set the named value in a list of name/value pairs, replacing the first match
// in place and removing any other matches
function setPair(pairs, name, value, caseInsensitive) {
    const n = caseInsensitive ? name.toLowerCase() : name
    const matches = (p) => (caseInsensitive ? toStr(p.name).toLowerCase() : p.name) === n
    const results = []
    let set = false
    for (const p of (pairs ?? [])) {
        if (matches(p)) {
            if (!set) {
                results.push({ name, value: toStr(value) })
                set = true
            }
        } else {
            results.push(p)
        }
    }
    if (!set) {
        results.push({ name, value: toStr(value) })
    }
    return results
}

function removePair(pairs, name, caseInsensitive) {
    if (!pairs) {
        return pairs
    }
    const n = caseInsensitive ? name.toLowerCase() : name
    return pairs.filter(p => (caseInsensitive ? toStr(p.name).toLowerCase() : p.name) !== n)
}

// Wrap a list of name/value pairs so that, in addition to array operations, values
// can be read, set or deleted by name (ex. request.headers['Accept'] = 'text/plain').
// Names matching array members (length, map, etc.) retain their array meaning
function namedPairs(pairs, caseInsensitive) {
    const isName = (target, prop) => typeof prop === 'string' && !(prop in target)
    const replace = (target, updated) => { target.splice(0, target.length, ...updated) }
    return new Proxy(pairs, {
        get: (target, prop, receiver) => {
            if (!isName(target, prop)) {
                return Reflect.get(target, prop, receiver)
            }
            const n = caseInsensitive ? prop.toLowerCase() : prop
            const match = target.find(p => p.disabled !== true
                && (caseInsensitive ? toStr(p.name).toLowerCase() : p.name) === n)
            return match?.value
        },
        set: (target, prop, value, receiver) => {
            if (!isName(target, prop)) {
                return Reflect.set(target, prop, value, receiver)
            }
            replace(target, setPair(target, prop, value, caseInsensitive))
            return true
        },
        deleteProperty: (target, prop) => {
            if (!isName(target, prop)) {
                return Reflect.deleteProperty(target, prop)
            }
            replace(target, removePair(target, prop, caseInsensitive))
            return true
        },
    })
}

// Define a request property holding name/value pairs, accessible either as an array or by name;
// assigned values can be arrays of name/value pairs or { name: value } objects
function definePairsProperty(req, property, caseInsensitive) {
    let pairs = normalizePairs(req[property], property) ?? []
    let proxy = namedPairs(pairs, caseInsensitive)
    Object.defineProperty(req, property, {
        enumerable: true,
        configurable: true,
        get: () => proxy,
        set: (value) => {
            pairs = normalizePairs(value, property) ?? []
            proxy = namedPairs(pairs, caseInsensitive)
        },
    })
}

function attachHelpers(req) {
    definePairsProperty(req, 'headers', true)
    definePairsProperty(req, 'queryStringParams', false)
    const helpers = {
        setHeader: (name, value) => { req.headers = setPair(req.headers, name, value, true) },
        removeHeader: (name) => { req.headers = removePair(req.headers, name, true) },
        setQueryParam: (name, value) => { req.queryStringParams = setPair(req.queryStringParams, name, value, false) },
        removeQueryParam: (name) => { req.queryStringParams = removePair(req.queryStringParams, name, false) },
    }
    for (const [name, fn] of Object.entries(helpers)) {
        Object.defineProperty(req, name, { value: fn, enumerable: false, writable: true, configurable: true })
    }
    return req
}

function normalizePairs(pairs, label) {
    if (pairs === undefined || pairs === null) {
        return undefined
    }
    if (!Array.isArray(pairs)) {
        if (typeof pairs === 'object') {
            // Allow { name: value } objects as shorthand
            return Object.entries(pairs).map(([name, value]) => ({ name, value: toStr(value) }))
        }
        throw new Error(`request.${label} must be an array of name/value pairs`)
    }
    return pairs.map(p => {
        const result = { name: toStr(p.name), value: toStr(p.value) }
        if (p.disabled === true) {
            result.disabled = true
        }
        return result
    })
}

function normalizeBody(body) {
    if (body === undefined || body === null) {
        return undefined
    }
    switch (body.type) {
        case BodyType.JSON:
            return {
                type: body.type,
                data: typeof body.data === 'string' ? body.data : JSON.stringify(body.data)
            }
        case BodyType.Text:
        case BodyType.XML:
            return { type: body.type, data: toStr(body.data) }
        case BodyType.Form:
            return { type: body.type, data: normalizePairs(body.data, 'body.data') ?? [] }
        case BodyType.GraphQL: {
            const extensions = body.data?.extensions
            return {
                type: body.type,
                data: {
                    query: toStr(body.data?.query),
                    extensions: (extensions === undefined || extensions === null)
                        ? null
                        : (typeof extensions === 'string' ? extensions : JSON.stringify(extensions))
                }
            }
        }
        case BodyType.Raw:
            if (isBinary(body.data)) {
                return { type: body.type, data: base64.encode(body.data) }
            }
            if (typeof body.data !== 'string') {
                throw new Error('Raw body data must be a base64 encoded string, array, typed array or ArrayBuffer')
            }
            return { type: body.type, data: body.data }
        default:
            throw new Error(`Invalid request body type "${body.type}"`)
    }
}

function normalizeRequest(req) {
    if (typeof req !== 'object' || req === null) {
        throw new Error('request must be an object')
    }
    const result = {
        url: toStr(req.url),
    }
    if (req.method !== undefined && req.method !== null) {
        result.method = toStr(req.method)
    }
    const headers = normalizePairs(req.headers, 'headers')
    if (headers?.length) {
        result.headers = headers
    }
    const queryStringParams = normalizePairs(req.queryStringParams, 'queryStringParams')
    if (queryStringParams?.length) {
        result.queryStringParams = queryStringParams
    }
    const body = normalizeBody(req.body)
    if (body) {
        result.body = body
    }
    return result
}

runSetup = (request1, variables1, data1, output1, setupOffset1, setup) => {
    // Group setup scripts are not passed a request
    const hasRequest = request1 !== undefined && request1 !== null
    request = hasRequest ? attachHelpers(request1) : null
    scenario = variables1 ?? {}
    data = data1 ?? {}
    outputVars = output1 ?? {}
    logs = []

    $ = { ...scenario, ...outputVars, ...data } // output overrides scenario, data overrides both
    variables = $ // retain variables for consistency with test framework

    setupOffset = setupOffset1

    // Errors are returned (rather than thrown) so that logs are retained
    try {
        setup()
        return JSON.stringify({
            request: hasRequest ? normalizeRequest(request) : null,
            output: outputVars,
            logs: logs.length > 0 ? logs : undefined
        })
    } catch (e) {
        return JSON.stringify({
            error: e?.message ?? `${e}`,
            logs: logs.length > 0 ? logs : undefined
        })
    }
};

module.exports = runSetup
