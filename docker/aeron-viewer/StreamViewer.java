import io.aeron.Aeron;
import io.aeron.Publication;
import io.aeron.Subscription;
import io.aeron.logbuffer.FragmentHandler;
import org.agrona.concurrent.UnsafeBuffer;

import com.sun.net.httpserver.Headers;
import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;

import java.io.IOException;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.time.Instant;
import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicReference;

public final class StreamViewer {
    private static final int MAX_MESSAGES = 500;
    private static final List<String> MESSAGES = new CopyOnWriteArrayList<>();
    private static final List<SseClient> SSE_CLIENTS = new CopyOnWriteArrayList<>();

    private StreamViewer() {}

    public static void main(String[] args) throws Exception {
        String aeronDir = env("AERON_DIR", "/data/aeron");
        String channel = env("AERON_CHANNEL", "aeron:ipc");
        int streamId = Integer.parseInt(env("AERON_STREAM_ID", "1001"));
        int port = Integer.parseInt(env("PORT", "8080"));

        Aeron.Context ctx = new Aeron.Context().aeronDirectoryName(aeronDir);
        Aeron aeron = Aeron.connect(ctx);
        Subscription subscription = aeron.addSubscription(channel, streamId);
        Publication publication = aeron.addPublication(channel, streamId);

        FragmentHandler handler = (buffer, offset, length, header) -> {
            byte[] bytes = new byte[length];
            buffer.getBytes(offset, bytes);
            String payload = new String(bytes, StandardCharsets.UTF_8);
            String entry = Instant.now() + " | session=" + header.sessionId() + " | " + payload;
            addMessage(entry);
        };

        Executors.newSingleThreadExecutor().submit(() -> {
            while (!Thread.currentThread().isInterrupted()) {
                subscription.poll(handler, 10);
                try {
                    Thread.sleep(5);
                } catch (InterruptedException e) {
                    Thread.currentThread().interrupt();
                }
            }
        });

        HttpServer server = HttpServer.create(new InetSocketAddress(port), 0);
        AtomicReference<String> config = new AtomicReference<>(channel + " / stream " + streamId);

        server.createContext("/", exchange -> serveHtml(exchange, config.get()));
        server.createContext("/api/messages", StreamViewer::serveMessages);
        server.createContext("/api/stream", StreamViewer::serveSse);
        server.createContext("/api/publish", exchange -> handlePublish(exchange, publication));
        server.setExecutor(Executors.newCachedThreadPool());
        server.start();

        System.out.println("Aeron viewer ready on port " + port);
        System.out.println("Channel: " + channel + ", streamId: " + streamId + ", dir: " + aeronDir);
    }

    private static String env(String key, String fallback) {
        String value = System.getenv(key);
        return value == null || value.isBlank() ? fallback : value;
    }

    private static void addMessage(String entry) {
        synchronized (MESSAGES) {
            MESSAGES.add(entry);
            while (MESSAGES.size() > MAX_MESSAGES) {
                MESSAGES.remove(0);
            }
        }
        broadcast(entry);
    }

    private static void broadcast(String entry) {
        String payload = "data: " + jsonString(entry) + "\n\n";
        for (SseClient client : SSE_CLIENTS) {
            client.send(payload);
        }
    }

    private static void serveHtml(HttpExchange exchange, String config) throws IOException {
        String html =
                """
                <!doctype html>
                <html>
                <head>
                  <meta charset="utf-8"/>
                  <title>Aeron Stream Viewer</title>
                  <style>
                    body { font-family: sans-serif; margin: 24px; background: #0f172a; color: #e2e8f0; }
                    h1 { margin-bottom: 8px; }
                    .meta { color: #94a3b8; margin-bottom: 16px; }
                    form { display: flex; gap: 8px; margin-bottom: 16px; }
                    input, button { padding: 10px; border-radius: 8px; border: 1px solid #334155; }
                    input { flex: 1; background: #1e293b; color: #e2e8f0; }
                    button { background: #2563eb; color: white; cursor: pointer; }
                    #messages { background: #111827; border: 1px solid #334155; border-radius: 12px; padding: 12px; height: 70vh; overflow: auto; }
                    .msg { padding: 8px 0; border-bottom: 1px solid #1f2937; font-family: monospace; white-space: pre-wrap; }
                  </style>
                </head>
                <body>
                  <h1>Aeron Stream Viewer</h1>
                  <div class="meta">Watching: %s</div>
                  <form id="publish-form">
                    <input id="message" placeholder="Type a message to publish..." required />
                    <button type="submit">Publish</button>
                  </form>
                  <div id="messages"></div>
                  <script>
                    const box = document.getElementById('messages');
                    function addLine(text) {
                      const div = document.createElement('div');
                      div.className = 'msg';
                      div.textContent = text;
                      box.prepend(div);
                    }
                    fetch('/api/messages').then(r => r.json()).then(items => items.reverse().forEach(addLine));
                    const source = new EventSource('/api/stream');
                    source.onmessage = (event) => addLine(JSON.parse(event.data));
                    document.getElementById('publish-form').addEventListener('submit', async (e) => {
                      e.preventDefault();
                      const input = document.getElementById('message');
                      const message = input.value.trim();
                      if (!message) return;
                      await fetch('/api/publish', {
                        method: 'POST',
                        headers: { 'Content-Type': 'text/plain' },
                        body: message
                      });
                      input.value = '';
                    });
                  </script>
                </body>
                </html>
                """
                        .formatted(config);

        byte[] bytes = html.getBytes(StandardCharsets.UTF_8);
        exchange.getResponseHeaders().set("Content-Type", "text/html; charset=utf-8");
        exchange.sendResponseHeaders(200, bytes.length);
        try (OutputStream os = exchange.getResponseBody()) {
            os.write(bytes);
        }
    }

    private static void serveMessages(HttpExchange exchange) throws IOException {
        String json;
        synchronized (MESSAGES) {
            json = "[" + String.join(",", MESSAGES.stream().map(StreamViewer::jsonString).toList()) + "]";
        }
        writeJson(exchange, 200, json);
    }

    private static void serveSse(HttpExchange exchange) throws IOException {
        Headers headers = exchange.getResponseHeaders();
        headers.set("Content-Type", "text/event-stream");
        headers.set("Cache-Control", "no-cache");
        headers.set("Connection", "keep-alive");
        exchange.sendResponseHeaders(200, 0);

        SseClient client = new SseClient(exchange);
        SSE_CLIENTS.add(client);
        exchange.getRequestBody().close();
    }

    private static void handlePublish(HttpExchange exchange, Publication publication) throws IOException {
        if (!"POST".equalsIgnoreCase(exchange.getRequestMethod())) {
            writeJson(exchange, 405, "{\"error\":\"method not allowed\"}");
            return;
        }

        String message = new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8).trim();
        if (message.isEmpty()) {
            writeJson(exchange, 400, "{\"error\":\"empty message\"}");
            return;
        }

        byte[] bytes = message.getBytes(StandardCharsets.UTF_8);
        UnsafeBuffer buffer = new UnsafeBuffer(new byte[bytes.length]);
        buffer.putBytes(0, bytes);

        long result = publication.offer(buffer, 0, bytes.length);
        if (result < 0) {
            writeJson(exchange, 503, "{\"error\":\"publication not ready: " + result + "\"}");
            return;
        }

        writeJson(exchange, 200, "{\"ok\":true}");
    }

    private static void writeJson(HttpExchange exchange, int status, String json) throws IOException {
        byte[] bytes = json.getBytes(StandardCharsets.UTF_8);
        exchange.getResponseHeaders().set("Content-Type", "application/json");
        exchange.sendResponseHeaders(status, bytes.length);
        try (OutputStream os = exchange.getResponseBody()) {
            os.write(bytes);
        }
    }

    private static String jsonString(String value) {
        return "\"" + value.replace("\\", "\\\\").replace("\"", "\\\"") + "\"";
    }

    private static final class SseClient {
        private final HttpExchange exchange;
        private final OutputStream output;

        private SseClient(HttpExchange exchange) throws IOException {
            this.exchange = exchange;
            this.output = exchange.getResponseBody();
        }

        private void send(String payload) {
            try {
                output.write(payload.getBytes(StandardCharsets.UTF_8));
                output.flush();
            } catch (IOException ignored) {
                SSE_CLIENTS.remove(this);
                try {
                    exchange.close();
                } catch (Exception ignoredClose) {
                    // client disconnected
                }
            }
        }
    }
}
