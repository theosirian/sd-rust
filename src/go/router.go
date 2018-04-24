package main

import (
	"fmt"
	"io"
	"log"
	"net"
	"os"
)

type Type int

const (
	None Type = iota
	Router
	Subscriber
	Publisher
)

func publisher(conn net.Conn) {
	defer conn.Close()
	conn.Write([]byte{0x01, 0x03})
	label := fmt.Sprintf("%v / %s", conn.RemoteAddr(), "PUBLISHER")
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

func main() {
	if len(os.Args) == 1 {
		log.Fatalf("[router] NO PUBLISHER(S) SPECIFIED, EXITING")
	} else {
		for _, port := range os.Args[1:] {
			log.Printf("[router] CONNECTING TO PUBLISHER %v", port)
			conn, err := net.Dial("tcp", fmt.Sprintf("127.0.0.1:%s", port))
			if err != nil {
				log.Fatalf("[router] ERROR: %v", err)
			}
			go publisher(conn)
		}
		l, err := net.Listen("tcp", ":13002")
		if err != nil {
			log.Fatalf("[router] ERROR: %v", err)
		}
		defer l.Close()
		log.Printf("[router] READY TO ACCEPT CONNECTIONS")
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
}
