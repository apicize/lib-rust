const runTestSuite = require('../index')

test('processes tag substitution', () => {
    let response = runTestSuite({}, {}, {}, {test1: 'test-123', value: 100}, {}, 0, () => {
        describe('test', () => {
            it('should be ok', () => {
                tag('{{test1}}')
                expect(data.value).to.equal(100)
            })
        })
    })
    results = JSON.parse(response).results
    testBehavior = results[0]
    testOk = testBehavior.children[0]
    expect(testBehavior.tag).to.be.undefined
    expect(testOk.tag).to.equal('test-123')
    expect(testOk.success).to.equal(true)
})


test('$ precedence is scenario < output < data', () => {
    const response = runTestSuite({}, {}, { a: 'scenario', b: 'scenario', c: 'scenario' },
        { c: 'data' }, { b: 'output', c: 'output' }, 0, () => {
            output('a2', $.a)
            output('b2', $.b)
            output('c2', $.c)
        })
    const result = JSON.parse(response).output
    expect(result.a2).to.equal('scenario')
    expect(result.b2).to.equal('output')
    expect(result.c2).to.equal('data')
})

// Note: describe/it are overridden by the framework, so base64 tests are not grouped

// Run helpers inside the test sandbox and return results via output
const runBase64 = (fn) => JSON.parse(runTestSuite({}, {}, {}, {}, {}, 0, fn)).output

test('base64: btoa and atob match Node for binary strings', () => {
    const binary = 'hi\u0000ÿ\u0080!'
    const result = runBase64(() => {
        output('encoded', btoa(binary))
        output('decoded', atob(btoa(binary)))
        output('unpadded', atob('aGk'))
    })
    expect(result.encoded).to.equal(Buffer.from(binary, 'latin1').toString('base64'))
    expect(result.decoded).to.equal(binary)
    expect(result.unpadded).to.equal('hi')
})

test('base64: btoa rejects non-Latin1 characters', () => {
    expect(() => runBase64(() => btoa('€'))).to.throw(/Latin1/)
})

test('base64: encodes and decodes UTF-8 text and bytes', () => {
    const result = runBase64(() => {
        output('text', base64.encode('héllo €'))
        output('bytes', base64.encode(new Uint8Array([0, 1, 254, 255])))
        output('decodedText', base64.decodeText(base64.encode('héllo €')))
        output('decodedBytes', Array.from(base64.decode('AAH+/w==')))
    })
    expect(result.text).to.equal(Buffer.from('héllo €', 'utf8').toString('base64'))
    expect(result.bytes).to.equal('AAH+/w==')
    expect(result.decodedText).to.equal('héllo €')
    expect(result.decodedBytes).to.deep.equal([0, 1, 254, 255])
})

test('base64: can be used in test assertions', () => {
    const suite = runTestSuite({}, { body: { text: 'aGVsbG8=' } }, {}, {}, {}, 0, () => {
        describe('decode', () => {
            it('decodes the body', () => {
                expect(atob(response.body.text)).to.equal('hello')
            })
        })
    })
    const decode = JSON.parse(suite).results.find(r => r.name === 'decode')
    expect(decode.children[0].success).to.equal(true)
})
