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
