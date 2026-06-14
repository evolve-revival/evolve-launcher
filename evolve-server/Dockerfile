FROM golang:alpine AS build
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -o evolve-server ./cmd/server

FROM alpine:latest
WORKDIR /app
COPY --from=build /app/evolve-server .
EXPOSE 8080
CMD ["./evolve-server"]
