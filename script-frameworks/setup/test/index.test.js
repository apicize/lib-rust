const runSetup = require('../index')

const baseRequest = () => ({
    url: 'https://example.com/{{path}}',
    method: 'GET',
    headers: [
        { name: 'Accept', value: 'application/json' },
        { name: 'X-Old', value: 'old', disabled: true },
    ],
    queryStringParams: [
        { name: 'a', value: '1' },
    ],
    body: { type: 'JSON', data: '{"value":1}' },
})

test('passes unmodified request through', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => { }))
    expect(result.request).toEqual(baseRequest())
    expect(result.output).toEqual({})
    expect(result.logs).toBeUndefined()
})

test('updates url and method', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request.url = 'https://other.com/'
        request.method = 'POST'
    }))
    expect(result.request.url).toEqual('https://other.com/')
    expect(result.request.method).toEqual('POST')
})

test('sets and removes headers case-insensitively', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request.setHeader('accept', 'text/plain')
        request.setHeader('X-New', 123)
        request.removeHeader('x-old')
    }))
    expect(result.request.headers).toEqual([
        { name: 'accept', value: 'text/plain' },
        { name: 'X-New', value: '123' },
    ])
})

test('sets and removes query string parameters', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request.setQueryParam('b', 2)
        request.removeQueryParam('a')
    }))
    expect(result.request.queryStringParams).toEqual([{ name: 'b', value: '2' }])
})

test('adds headers and query string parameters when none exist', () => {
    const result = JSON.parse(runSetup({ url: 'https://example.com' }, {}, {}, {}, 0, () => {
        request.setHeader('X-Test', 'yes')
        request.setQueryParam('q', 'x')
    }))
    expect(result.request.headers).toEqual([{ name: 'X-Test', value: 'yes' }])
    expect(result.request.queryStringParams).toEqual([{ name: 'q', value: 'x' }])
})

test('accepts object shorthand for headers', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request.headers = { 'X-A': 'a', 'X-B': 2 }
    }))
    expect(result.request.headers).toEqual([
        { name: 'X-A', value: 'a' },
        { name: 'X-B', value: '2' },
    ])
})

test('serializes JSON body objects', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        const body = JSON.parse(request.body.data)
        body.value += 1
        request.body.data = body
    }))
    expect(result.request.body).toEqual({ type: 'JSON', data: '{"value":2}' })
})

test('supports switching body types', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request.body = { type: BodyType.Form, data: { a: 1, b: 'two' } }
    }))
    expect(result.request.body).toEqual({
        type: 'Form',
        data: [{ name: 'a', value: '1' }, { name: 'b', value: 'two' }]
    })
})

test('serializes GraphQL extensions', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request.body = { type: BodyType.GraphQL, data: { query: '{ a }', extensions: { x: 1 } } }
    }))
    expect(result.request.body).toEqual({
        type: 'GraphQL',
        data: { query: '{ a }', extensions: '{"x":1}' }
    })
})

test('removes body', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request.body = undefined
    }))
    expect(result.request.body).toBeUndefined()
})

test('rejects invalid body type', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request.body = { type: 'Bogus', data: '' }
    }))
    expect(result.error).toEqual('Invalid request body type "Bogus"')
    expect(result.request).toBeUndefined()
})

test('returns errors with logs', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        console.log('before')
        throw new Error('boom')
    }))
    expect(result.error).toEqual('boom')
    expect(result.logs.length).toEqual(1)
    expect(result.logs[0]).toMatch(/\[log\] before$/)
})

test('exposes scenario, data, output and $', () => {
    const result = JSON.parse(runSetup(baseRequest(), { s: 'scenario' }, { d: 'data' }, { o: 'output' }, 0, () => {
        request.setHeader('X-S', scenario.s)
        request.setHeader('X-D', data.d)
        request.setHeader('X-All', `${$.s}-${$.d}-${$.o}`)
    }))
    expect(result.request.headers.slice(2)).toEqual([
        { name: 'X-S', value: 'scenario' },
        { name: 'X-D', value: 'data' },
        { name: 'X-All', value: 'scenario-data-output' },
    ])
})

test('outputs values', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, { existing: 1, removed: 2 }, 0, () => {
        output('path', 'abc')
        output('removed', undefined)
    }))
    expect(result.output).toEqual({ existing: 1, path: 'abc' })
})

test('records logs and resets them between runs', () => {
    let result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        console.info('hello %s', 'world')
    }))
    expect(result.logs.length).toEqual(1)
    expect(result.logs[0]).toMatch(/\[info\] hello world$/)
    result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => { }))
    expect(result.logs).toBeUndefined()
})

test('supports jsonpath', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, { items: [{ id: 5 }] }, {}, 0, () => {
        output('id', data.jp('$.items[0].id')[0])
    }))
    expect(result.output.id).toEqual(5)
})

test('supports null request (group setup)', () => {
    const result = JSON.parse(runSetup(null, { a: 1 }, {}, {}, 0, () => {
        output('b', $.a + 1)
    }))
    expect(result.request).toBeNull()
    expect(result.output).toEqual({ b: 2 })
})

test('throws if request is set to null', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request = null
    }))
    expect(result.error).toEqual('request must be an object')
})

test('accepts a replaced request object', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request = { url: 'https://new.com', method: 'PUT' }
    }))
    expect(result.request).toEqual({ url: 'https://new.com', method: 'PUT' })
})

describe('base64', () => {
    // Run helpers inside the setup sandbox and return results via output
    const run = (fn) => JSON.parse(runSetup(null, {}, {}, {}, 0, fn))

    test('btoa and atob match Node for binary strings', () => {
        const binary = 'hi\u0000ÿ\u0080!'
        const result = run(() => {
            output('encoded', btoa(binary))
            output('decoded', atob(btoa(binary)))
            output('unpadded', atob('aGk'))
        })
        expect(result.output.encoded).toEqual(Buffer.from(binary, 'latin1').toString('base64'))
        expect(result.output.decoded).toEqual(binary)
        expect(result.output.unpadded).toEqual('hi')
    })

    test('btoa rejects non-Latin1 characters', () => {
        const result = run(() => btoa('€'))
        expect(result.error).toMatch(/Latin1/)
    })

    test('encodes and decodes UTF-8 text and bytes', () => {
        const result = run(() => {
            output('text', base64.encode('héllo €'))
            output('bytes', base64.encode(new Uint8Array([0, 1, 254, 255])))
            output('array', base64.encode([104, 105]))
            output('decodedText', base64.decodeText(base64.encode('héllo €')))
            output('decodedBytes', Array.from(base64.decode('AAH+/w==')))
        })
        expect(result.output.text).toEqual(Buffer.from('héllo €', 'utf8').toString('base64'))
        expect(result.output.bytes).toEqual('AAH+/w==')
        expect(result.output.array).toEqual('aGk=')
        expect(result.output.decodedText).toEqual('héllo €')
        expect(result.output.decodedBytes).toEqual([0, 1, 254, 255])
    })

    test('rejects invalid base64', () => {
        expect(run(() => base64.decode('a$bc')).error).toEqual('Invalid base64 character "$"')
        expect(run(() => base64.decode('abcde')).error).toEqual('Invalid base64 string')
    })

    test('encodes binary Raw body data', () => {
        const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
            request.body = { type: BodyType.Raw, data: new Uint8Array([1, 2, 3]) }
        }))
        expect(result.request.body).toEqual({ type: 'Raw', data: 'AQID' })
    })
})

test('$ precedence is scenario < output < data', () => {
    const result = JSON.parse(runSetup(null, { a: 'scenario', b: 'scenario', c: 'scenario' },
        { c: 'data' }, { b: 'output', c: 'output' }, 0, () => {
            output('a2', $.a)
            output('b2', $.b)
            output('c2', $.c)
        }))
    expect(result.output.a2).toEqual('scenario')
    expect(result.output.b2).toEqual('output')
    expect(result.output.c2).toEqual('data')
})

test('sets, reads and deletes headers by name', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request.headers['foo'] = '888'
        request.headers['ACCEPT'] = 'text/plain'
        if (request.headers['accept'] !== 'text/plain') throw new Error('header not readable by name')
        delete request.headers['x-old']
    }))
    expect(result.error).toBeUndefined()
    expect(result.request.headers).toEqual([
        { name: 'ACCEPT', value: 'text/plain' },
        { name: 'foo', value: '888' },
    ])
})

test('sets headers and query string parameters by name when none exist', () => {
    const result = JSON.parse(runSetup({ url: 'https://example.com' }, {}, {}, {}, 0, () => {
        request.headers['foo'] = 888
        request.queryStringParams['q'] = 'x'
    }))
    expect(result.request.headers).toEqual([{ name: 'foo', value: '888' }])
    expect(result.request.queryStringParams).toEqual([{ name: 'q', value: 'x' }])
})

test('query string parameter names are case-sensitive', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request.queryStringParams['A'] = '2'
    }))
    expect(result.request.queryStringParams).toEqual([
        { name: 'a', value: '1' },
        { name: 'A', value: '2' },
    ])
})

test('retains array behavior and name access after assignment', () => {
    const result = JSON.parse(runSetup(baseRequest(), {}, {}, {}, 0, () => {
        request.headers.push({ name: 'X-Pushed', value: 'p' })
        request.headers = { 'X-Obj': 'o' }
        request.headers['X-Named'] = 'n'
        if (request.headers.length !== 2) throw new Error('unexpected length')
    }))
    expect(result.error).toBeUndefined()
    expect(result.request.headers).toEqual([
        { name: 'X-Obj', value: 'o' },
        { name: 'X-Named', value: 'n' },
    ])
})

test('omits headers when none are left', () => {
    const result = JSON.parse(runSetup({ url: 'https://example.com', headers: [{ name: 'a', value: '1' }] }, {}, {}, {}, 0, () => {
        delete request.headers['a']
    }))
    expect(result.request.headers).toBeUndefined()
})
