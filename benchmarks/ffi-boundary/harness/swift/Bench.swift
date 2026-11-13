// Compiled once with each binding. Prints one tab-separated line per scenario.
import Foundation

final class Sink: Listener, @unchecked Sendable {
    var seen = 0
    func commit(dirty: [Int64], canBack: Bool) { seen &+= dirty.count }
    func schedulePump() {}
}

nonisolated(unsafe) var blackhole: UInt64 = 0

func measure(_ name: String, _ iterations: Int, _ body: () -> Void) {
    var samples: [Double] = []
    for _ in 0..<31 {
        let start = DispatchTime.now().uptimeNanoseconds
        for _ in 0..<iterations { body() }
        let end = DispatchTime.now().uptimeNanoseconds
        samples.append(Double(end - start) / Double(iterations))
    }
    samples.sort()
    let median = samples[samples.count / 2]
    let low = samples[samples.count / 10]
    let high = samples[samples.count * 9 / 10]
    print("\(name)\t\(String(format: "%.1f", median))\t\(String(format: "%.1f", low))\t\(String(format: "%.1f", high))")
}

@main
struct Main {
    static func main() {
        setvbuf(stdout, nil, _IOLBF, 0)
        let sink = Sink()
        let node = mount(listener: sink)
        measure("noop", 200_000) { noop() }
        measure("echo", 200_000) { blackhole &+= echo(value: 7) }
        measure("row", 100_000) { blackhole &+= row(id: 7).value }
        measure("rows_30", 20_000) { blackhole &+= UInt64(rows(start: 0, count: 30).count) }
        measure("rows_1000", 1_000) { blackhole &+= UInt64(rows(start: 0, count: 1000).count) }
        measure("entries_30", 20_000) { blackhole &+= UInt64(entries(count: 30).count) }
        measure("dirty_30", 20_000) { blackhole &+= UInt64(dirty(count: 30).count) }
        measure("article", 20_000) { blackhole &+= UInt64(article(id: 3).body.utf8.count) }
        measure("model", 100_000) { blackhole &+= value(node: node) }
        measure("tap_with_commit", 50_000) { precondition(tap(node: node)) }
        precondition(sink.seen > 0)
        print("# blackhole \(blackhole)")
    }
}
