/******************************************************************
 * Base64 helpers shared by the test and setup frameworks
 * (V8 does not include btoa/atob or TextEncoder)
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

module.exports = { isBinary }
