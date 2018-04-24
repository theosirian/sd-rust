package main

import (
	"fmt"
	"io"
	"log"
	"net"
)

type Type int

const (
	None Type = iota
	Router
	Subscriber
	Publisher
)

func main() {
	l, err := net.Listen("tcp", ":13002")
	if err != nil {
		log.Fatal(err)
	}
	defer l.Close()
	for {
		conn, err := l.Accept()
		if err != nil {
			log.Fatalf("[router] ERROR: %v", err)
		}
		go func(c net.Conn) {
			defer c.Close()
			label := fmt.Sprintf("%v", c.RemoteAddr())
			log.Printf("[router] ACCEPT %v", label)
			state := None
			buf := make([]byte, 0, 1024)
			tmp := make([]byte, 256)
			for {
				n, err := conn.Read(tmp)
				if err != nil {
					if err != io.EOF {
						log.Fatal(err)
					}
					break
				}
				buf = append(buf, tmp[:n]...)
			Read:
				for len(buf) > 0 {
					if state == None {
						var first byte
						first, buf = buf[0], buf[1:]
						switch first {
						case 0:
							state = Router
							log.Printf("[%v] IDENTIFIED AS ROUTER", label)
							label = fmt.Sprintf("%s / %s", label, "ROUTER")
						case 1:
							state = Subscriber
							log.Printf("[%v] IDENTIFIED AS SUBSCRIBER", label)
							label = fmt.Sprintf("%s / %s", label, "SUBSCRIBER")
						case 2:
							state = Publisher
							log.Printf("[%v] IDENTIFIED AS PUBLISHER", label)
							label = fmt.Sprintf("%s / %s", label, "PUBLISHER")
						default:
							state = None
							log.Printf("[%v] FAILED IDENTIFICATION", label)
							label = fmt.Sprintf("%s / %s", label, "FAILED")
						}
					} else {
						var size int
						size = int(buf[0])
						if len(buf) > size {
							var msg []byte
							size, msg, buf = int(buf[0]), buf[1:size+1], buf[size+1:]
							log.Printf("[%v] RECV [%v] = [%v]; tail = [%v]", label, size, msg, buf)
						} else {
							break Read
						}
					}
				}
			}
		}(conn)
	}
}
