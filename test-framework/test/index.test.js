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

